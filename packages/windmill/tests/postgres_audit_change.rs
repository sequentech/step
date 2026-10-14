// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A change a person makes through Hasura writes its AdminChange entry to
//! `audit_log_outbox` in the same transaction (`sequent_backend.audit_change`):
//! who made it, the object, and each changed field with its old and new value.
//! Step's own writes, cascades and unchanged rows write nothing.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const USER_ID: &str = "8f2b7a52-5a0c-4c0e-9d57-3f1f4a8f6f10";
const ADMIN_USER: &str = "admin-user";
/// Texts longer than this are logged as their SHA-256.
const MAX_VALUE_BYTES: usize = 65536;

#[derive(Debug)]
struct Entry {
    election_event_id: Option<Uuid>,
    election_id: Option<Uuid>,
    area_id: Option<Uuid>,
    user_id: Option<String>,
    username: Option<String>,
    statement_kind: String,
    event_type: String,
    log_type: String,
    description: String,
    details: Value,
}

impl Entry {
    fn action(&self) -> &str {
        self.details["action"].as_str().unwrap()
    }

    fn changes(&self) -> &Vec<Value> {
        self.details["changes"].as_array().unwrap()
    }

    fn paths(&self) -> Vec<&str> {
        self.changes()
            .iter()
            .map(|change| change["path"].as_str().unwrap())
            .collect()
    }

    fn change(&self, path: &str) -> &Value {
        self.changes()
            .iter()
            .find(|change| change["path"] == path)
            .unwrap_or_else(|| panic!("no change of {path} in {:?}", self.paths()))
    }
}

/// The tenant's entries in posting order.
async fn entries(tx: &Transaction<'_>, tenant: Uuid) -> Vec<Entry> {
    tx.query(
        "SELECT election_event_id, election_id, area_id, user_id, username, statement_kind,
                event_type, log_type, description, details
         FROM sequent_backend.audit_log_outbox WHERE tenant_id = $1 ORDER BY seq",
        &[&tenant],
    )
    .await
    .unwrap()
    .into_iter()
    .map(|row| Entry {
        election_event_id: row.get(0),
        election_id: row.get(1),
        area_id: row.get(2),
        user_id: row.get(3),
        username: row.get(4),
        statement_kind: row.get(5),
        event_type: row.get(6),
        log_type: row.get(7),
        description: row.get(8),
        details: row.get(9),
    })
    .collect()
}

async fn one_entry(tx: &Transaction<'_>, tenant: Uuid) -> Entry {
    let mut found = entries(tx, tenant).await;
    assert_eq!(found.len(), 1, "{found:#?}");
    found.remove(0)
}

/// The rest of the transaction runs as Hasura does for `role`.
async fn act_as(tx: &Transaction<'_>, role: &str, tenant: Uuid) {
    let session = json!({
        "x-hasura-role": role,
        "x-hasura-user-id": USER_ID,
        "x-hasura-tenant-id": tenant,
    });
    set_session(tx, &session.to_string()).await;
}

async fn set_session(tx: &Transaction<'_>, session: &str) {
    tx.execute("SELECT set_config('hasura.user', $1, true)", &[&session])
        .await
        .unwrap();
}

fn slug(tenant: Uuid) -> String {
    format!("tenant-{tenant}")
}

async fn insert_tenant(tx: &Transaction<'_>, settings: &Value) -> Uuid {
    let tenant = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug, settings) VALUES ($1, $2, $3)",
        &[&tenant, &slug(tenant), settings],
    )
    .await
    .unwrap();
    tenant
}

async fn set_settings(tx: &Transaction<'_>, tenant: Uuid, settings: &Value) {
    tx.execute(
        "UPDATE sequent_backend.tenant SET settings = $2 WHERE id = $1",
        &[&tenant, settings],
    )
    .await
    .unwrap();
}

async fn insert_event(tx: &Transaction<'_>, tenant: Uuid, name: &str) -> Uuid {
    let event = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol, presentation)
         VALUES ($1, $2, 'RSA256', $3)",
        &[&event, &tenant, &json!({"i18n": {"en": {"name": name}}})],
    )
    .await
    .unwrap();
    event
}

async fn insert_election(tx: &Transaction<'_>, tenant: Uuid, event: Uuid, name: &str) -> Uuid {
    let election = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, presentation)
         VALUES ($1, $2, $3, $4)",
        &[
            &election,
            &tenant,
            &event,
            &json!({"i18n": {"en": {"name": name}}}),
        ],
    )
    .await
    .unwrap();
    election
}

async fn insert_area(tx: &Transaction<'_>, tenant: Uuid, event: Uuid, name: &str) -> Uuid {
    let area = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
         VALUES ($1, $2, $3, $4)",
        &[&area, &tenant, &event, &name],
    )
    .await
    .unwrap();
    area
}

async fn insert_template(tx: &Transaction<'_>, tenant: Uuid, body: &str) -> Uuid {
    let template = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.template
             (id, tenant_id, template, created_by, communication_method, type, alias)
         VALUES ($1, $2, $3, 'admin', 'EMAIL', 'BALLOT_RECEIPT', 'Receipt')",
        &[
            &template,
            &tenant,
            &json!({"email": {"subject": "Your receipt", "html_body": body}}),
        ],
    )
    .await
    .unwrap();
    template
}

async fn set_template_body(tx: &Transaction<'_>, template: Uuid, body: &str) {
    tx.execute(
        "UPDATE sequent_backend.template
         SET template = jsonb_set(template, '{email,html_body}', to_jsonb($2::text))
         WHERE id = $1",
        &[&template, &body],
    )
    .await
    .unwrap();
}

fn sha256_hex(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

#[tokio::test]
async fn a_settings_change_writes_one_tenant_log_entry_with_old_and_new_values() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = insert_tenant(
        &tx,
        &json!({
            "logs_timezone": "UTC",
            "gapi_key": "first-key",
            "gapi_email": "robot@example.com",
            "language_conf": {"default_language_code": "en", "enabled_language_codes": ["en"]},
        }),
    )
    .await;

    act_as(&tx, ADMIN_USER, tenant).await;
    set_settings(
        &tx,
        tenant,
        &json!({
            "logs_timezone": "Asia/Manila",
            "gapi_key": "second-key",
            "gapi_email": "robot@example.com",
            "language_conf": {"default_language_code": "tl", "enabled_language_codes": ["en"]},
        }),
    )
    .await;

    let entry = one_entry(&tx, tenant).await;
    assert_eq!(entry.election_event_id, None);
    assert_eq!(entry.election_id, None);
    assert_eq!(entry.area_id, None);
    assert_eq!(entry.user_id.as_deref(), Some(USER_ID));
    assert_eq!(entry.username, None);
    assert_eq!(entry.statement_kind, "AdminChange");
    assert_eq!(entry.event_type, "USER");
    assert_eq!(entry.log_type, "INFO");
    assert_eq!(
        entry.description,
        format!(
            "Updated tenant \"{}\": settings.gapi_key, \
             settings.language_conf.default_language_code, settings.logs_timezone",
            slug(tenant)
        )
    );
    assert_eq!(
        entry.details,
        json!({
            "object": {"type": "tenant", "id": tenant, "name": slug(tenant)},
            "action": "update",
            "actor": {"user_id": USER_ID, "username": null, "role": ADMIN_USER},
            "origin": "hasura",
            "changes": [
                {"path": "settings.gapi_key", "secret": true},
                {
                    "path": "settings.language_conf.default_language_code",
                    "old": "en",
                    "new": "tl",
                },
                {"path": "settings.logs_timezone", "old": "UTC", "new": "Asia/Manila"},
            ],
        })
    );
    assert!(!entry.details.to_string().contains("-key\""));
}

#[tokio::test]
async fn a_change_that_is_rolled_back_leaves_no_entry() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let mut tx = client.transaction().await.unwrap();
    let before = json!({"logs_timezone": "UTC"});
    let tenant = insert_tenant(&tx, &before).await;
    act_as(&tx, ADMIN_USER, tenant).await;

    let request = tx.transaction().await.unwrap();
    set_settings(&request, tenant, &json!({"logs_timezone": "Asia/Manila"})).await;
    assert_eq!(entries(&request, tenant).await.len(), 1);
    // The request's second mutation names a tenant that does not exist.
    let second = request
        .execute(
            "INSERT INTO sequent_backend.election_event (tenant_id, encryption_protocol)
             VALUES ($1, 'RSA256')",
            &[&Uuid::new_v4()],
        )
        .await;
    assert!(second.is_err());
    request.rollback().await.unwrap();

    assert!(entries(&tx, tenant).await.is_empty());
    let settings: Value = tx
        .query_one(
            "SELECT settings FROM sequent_backend.tenant WHERE id = $1",
            &[&tenant],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(settings, before);
}

#[tokio::test]
async fn writes_outside_a_persons_hasura_session_write_no_entry() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = insert_tenant(&tx, &json!({"logs_timezone": "UTC"})).await;
    let event = insert_event(&tx, tenant, "National").await;
    insert_election(&tx, tenant, event, "Senate").await;
    set_settings(&tx, tenant, &json!({"logs_timezone": "Asia/Manila"})).await;
    assert!(entries(&tx, tenant).await.is_empty(), "direct SQL");

    set_session(&tx, "").await;
    set_settings(&tx, tenant, &json!({"logs_timezone": "Asia/Tokyo"})).await;
    assert!(entries(&tx, tenant).await.is_empty(), "session reset");

    for role in ["service-account", "admin"] {
        act_as(&tx, role, tenant).await;
        set_settings(&tx, tenant, &json!({"logs_timezone": role})).await;
        insert_area(&tx, tenant, event, "North").await;
        tx.execute(
            "DELETE FROM sequent_backend.area WHERE election_event_id = $1",
            &[&event],
        )
        .await
        .unwrap();
        assert!(entries(&tx, tenant).await.is_empty(), "{role}");
    }
}

#[tokio::test]
async fn any_other_role_is_logged_with_its_name() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = insert_tenant(&tx, &json!({})).await;
    let event = insert_event(&tx, tenant, "National").await;

    act_as(&tx, "area-write", tenant).await;
    insert_area(&tx, tenant, event, "North").await;

    let entry = one_entry(&tx, tenant).await;
    assert_eq!(
        entry.details["actor"],
        json!({"user_id": USER_ID, "username": null, "role": "area-write"})
    );
}

#[tokio::test]
async fn an_event_row_goes_to_its_event_log_with_its_election_and_area() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = insert_tenant(&tx, &json!({})).await;
    let event = insert_event(&tx, tenant, "National").await;

    act_as(&tx, ADMIN_USER, tenant).await;
    let election = insert_election(&tx, tenant, event, "Senate").await;
    let area = insert_area(&tx, tenant, event, "North").await;
    tx.execute("DELETE FROM sequent_backend.area WHERE id = $1", &[&area])
        .await
        .unwrap();

    let found = entries(&tx, tenant).await;
    assert_eq!(found.len(), 3, "{found:#?}");
    let created = &found[0];
    assert_eq!(created.election_event_id, Some(event));
    assert_eq!(created.election_id, Some(election));
    assert_eq!(created.area_id, None);
    assert_eq!(created.description, "Created election \"Senate\"");
    assert_eq!(created.action(), "create");
    assert_eq!(
        created.details["object"],
        json!({"type": "election", "id": election, "name": "Senate"})
    );
    assert_eq!(
        created.change("presentation.i18n.en.name"),
        &json!({"path": "presentation.i18n.en.name", "new": "Senate"})
    );
    assert_eq!(
        created.change("election_event_id"),
        &json!({"path": "election_event_id", "new": event})
    );
    // Unset columns and the row's timestamps are not values the person gave.
    for absent in ["description", "created_at", "last_updated_at"] {
        assert!(!created.paths().contains(&absent), "{absent}");
    }

    let area_created = &found[1];
    assert_eq!(area_created.election_event_id, Some(event));
    assert_eq!(area_created.election_id, None);
    assert_eq!(area_created.area_id, Some(area));
    assert_eq!(area_created.description, "Created area \"North\"");

    let deleted = &found[2];
    assert_eq!(deleted.election_event_id, Some(event));
    assert_eq!(deleted.area_id, Some(area));
    assert_eq!(deleted.description, "Deleted area \"North\"");
    assert_eq!(deleted.action(), "delete");
    assert_eq!(
        deleted.change("name"),
        &json!({"path": "name", "old": "North"})
    );
}

#[tokio::test]
async fn creating_and_archiving_an_event_are_tenant_log_entries() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = insert_tenant(&tx, &json!({})).await;

    act_as(&tx, ADMIN_USER, tenant).await;
    let event = insert_event(&tx, tenant, "National").await;
    for archived in [true, false] {
        tx.execute(
            "UPDATE sequent_backend.election_event SET is_archived = $2 WHERE id = $1",
            &[&event, &archived],
        )
        .await
        .unwrap();
    }

    let found = entries(&tx, tenant).await;
    assert_eq!(found.len(), 3, "{found:#?}");
    for entry in &found {
        assert_eq!(entry.election_event_id, None);
        assert_eq!(
            entry.details["object"],
            json!({"type": "election_event", "id": event, "name": "National"})
        );
    }
    assert_eq!(found[0].action(), "create");
    assert_eq!(found[0].description, "Created election event \"National\"");
    assert_eq!(found[1].action(), "archive");
    assert_eq!(found[1].description, "Archived election event \"National\"");
    assert_eq!(
        found[1].details["changes"],
        json!([{"path": "is_archived", "old": false, "new": true}])
    );
    assert_eq!(found[2].action(), "unarchive");
    assert_eq!(
        found[2].description,
        "Unarchived election event \"National\""
    );
    assert_eq!(
        found[2].details["changes"],
        json!([{"path": "is_archived", "old": true, "new": false}])
    );
}

#[tokio::test]
async fn archiving_with_another_change_writes_one_entry_to_each_log() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = insert_tenant(&tx, &json!({})).await;
    let event = insert_event(&tx, tenant, "National").await;

    act_as(&tx, ADMIN_USER, tenant).await;
    tx.execute(
        "UPDATE sequent_backend.election_event
         SET is_archived = true, description = 'Closed in May' WHERE id = $1",
        &[&event],
    )
    .await
    .unwrap();

    let found = entries(&tx, tenant).await;
    assert_eq!(found.len(), 2, "{found:#?}");
    assert_eq!(found[0].election_event_id, None);
    assert_eq!(found[0].action(), "archive");
    assert_eq!(found[0].paths(), ["is_archived"]);
    assert_eq!(found[1].election_event_id, Some(event));
    assert_eq!(found[1].action(), "update");
    assert_eq!(
        found[1].description,
        "Updated election event \"National\": description"
    );
    assert_eq!(
        found[1].details["changes"],
        json!([{"path": "description", "new": "Closed in May"}])
    );
}

#[tokio::test]
async fn an_event_change_without_archiving_goes_to_the_event_log_only() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = insert_tenant(&tx, &json!({})).await;
    let event = insert_event(&tx, tenant, "National").await;

    act_as(&tx, ADMIN_USER, tenant).await;
    tx.execute(
        "UPDATE sequent_backend.election_event
         SET presentation = jsonb_set(presentation, '{i18n,en,name}', '\"General\"')
         WHERE id = $1",
        &[&event],
    )
    .await
    .unwrap();

    let entry = one_entry(&tx, tenant).await;
    assert_eq!(entry.election_event_id, Some(event));
    assert_eq!(
        entry.description,
        "Updated election event \"General\": presentation.i18n.en.name"
    );
    assert_eq!(
        entry.details["changes"],
        json!([{"path": "presentation.i18n.en.name", "old": "National", "new": "General"}])
    );
}

#[tokio::test]
async fn deleting_an_event_writes_one_tenant_log_entry_and_none_for_its_cascade() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = insert_tenant(&tx, &json!({})).await;
    let event = insert_event(&tx, tenant, "National").await;
    let kept_event = insert_event(&tx, tenant, "Local").await;
    for of_event in [event, kept_event] {
        tx.execute(
            "INSERT INTO sequent_backend.certificate_authority
                 (tenant_id, election_event_id, common_name, subject, issuer_common_name, issuer,
                  not_before, not_after, fingerprint_sha256, serial_number, pem)
             VALUES ($1, $2, 'Root', 'CN=Root', 'Root', 'CN=Root', now(),
                     now() + interval '1 year', $3, '01', 'pem')",
            &[&tenant, &of_event, &sha256_hex(&of_event.to_string())],
        )
        .await
        .unwrap();
    }

    act_as(&tx, "election-event-delete", tenant).await;
    tx.execute(
        "DELETE FROM sequent_backend.election_event WHERE id = $1",
        &[&event],
    )
    .await
    .unwrap();

    let authorities: i64 = tx
        .query_one(
            "SELECT count(*) FROM sequent_backend.certificate_authority WHERE tenant_id = $1",
            &[&tenant],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(authorities, 1, "the event's authority went with it");
    let entry = one_entry(&tx, tenant).await;
    assert_eq!(entry.election_event_id, None);
    assert_eq!(entry.action(), "delete");
    assert_eq!(entry.description, "Deleted election event \"National\"");
    assert_eq!(
        entry.details["object"],
        json!({"type": "election_event", "id": event, "name": "National"})
    );
    assert_eq!(
        entry.change("encryption_protocol"),
        &json!({"path": "encryption_protocol", "old": "RSA256"})
    );

    // Deleting a certificate authority itself is the person's own statement.
    tx.execute(
        "DELETE FROM sequent_backend.certificate_authority WHERE election_event_id = $1",
        &[&kept_event],
    )
    .await
    .unwrap();
    let found = entries(&tx, tenant).await;
    assert_eq!(found.len(), 2, "{found:#?}");
    assert_eq!(found[1].election_event_id, Some(kept_event));
    assert_eq!(found[1].details["object"]["type"], "certificate_authority");
    assert_eq!(found[1].action(), "delete");
}

#[tokio::test]
async fn an_update_that_changes_nothing_writes_no_entry() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let settings = json!({"logs_timezone": "UTC", "sms": {"provider": "none"}});
    let tenant = insert_tenant(&tx, &settings).await;
    let event = insert_event(&tx, tenant, "National").await;

    act_as(&tx, ADMIN_USER, tenant).await;
    set_settings(&tx, tenant, &settings).await;
    // Only the row's own timestamp moves.
    tx.execute(
        "UPDATE sequent_backend.tenant SET updated_at = now() + interval '1 hour' WHERE id = $1",
        &[&tenant],
    )
    .await
    .unwrap();
    tx.execute(
        "UPDATE sequent_backend.election_event SET is_archived = is_archived WHERE id = $1",
        &[&event],
    )
    .await
    .unwrap();

    assert!(entries(&tx, tenant).await.is_empty());
}

#[tokio::test]
async fn json_values_are_compared_key_by_key_under_dotted_paths() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = insert_tenant(
        &tx,
        &json!({
            "sms": {"provider": {"name": "first", "region": "eu"}, "sender": "Step"},
            "help_links": [{"url": "https://example.com/a", "title": "A"}],
            "removed": {"kept": 1},
            "mail": true,
            "cleared": "value",
            "unchanged": {"deep": [1, 2, 3]},
        }),
    )
    .await;

    act_as(&tx, ADMIN_USER, tenant).await;
    set_settings(
        &tx,
        tenant,
        &json!({
            "sms": {"provider": {"name": "second", "region": "eu"}, "retries": 3},
            "help_links": [
                {"url": "https://example.com/a", "title": "A"},
                {"url": "https://example.com/b", "title": "B"},
            ],
            "mail": {"enabled": true},
            "cleared": null,
            "unchanged": {"deep": [1, 2, 3]},
        }),
    )
    .await;

    let entry = one_entry(&tx, tenant).await;
    assert_eq!(
        entry.details["changes"],
        json!([
            {"path": "settings.cleared", "old": "value", "new": null},
            {
                "path": "settings.help_links",
                "old": [{"url": "https://example.com/a", "title": "A"}],
                "new": [
                    {"url": "https://example.com/a", "title": "A"},
                    {"url": "https://example.com/b", "title": "B"},
                ],
            },
            {"path": "settings.mail", "old": true, "new": {"enabled": true}},
            {"path": "settings.removed.kept", "old": 1},
            {"path": "settings.sms.provider.name", "old": "first", "new": "second"},
            {"path": "settings.sms.retries", "new": 3},
            {"path": "settings.sms.sender", "old": "Step"},
        ])
    );
}

#[tokio::test]
async fn a_column_set_or_cleared_lists_its_keys() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = insert_tenant(&tx, &json!({"sms": {"sender": "Step"}})).await;

    act_as(&tx, ADMIN_USER, tenant).await;
    tx.execute(
        "UPDATE sequent_backend.tenant SET settings = NULL, annotations = '{}' WHERE id = $1",
        &[&tenant],
    )
    .await
    .unwrap();

    let entry = one_entry(&tx, tenant).await;
    assert_eq!(
        entry.details["changes"],
        json!([
            {"path": "annotations", "new": {}},
            {"path": "settings.sms.sender", "old": "Step"},
        ])
    );
}

#[tokio::test]
async fn secrets_are_never_logged() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = Uuid::new_v4();

    // The session's tenant is the one being created.
    act_as(&tx, ADMIN_USER, tenant).await;
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug, settings) VALUES ($1, $2, $3)",
        &[
            &tenant,
            &slug(tenant),
            &json!({
                "gapi_key": "hunter2-gapi",
                "smtp": {"host": "mail.example.com", "smtp_password": "hunter2-smtp"},
                "credentials": {"API_KEY": "hunter2-api", "REPORT_USERNAME": "hunter2-user"},
                "password_policy": {"min_length": 12},
            }),
        ],
    )
    .await
    .unwrap();
    set_settings(
        &tx,
        tenant,
        &json!({
            "gapi_key": "hunter2-gapi",
            "smtp": {"host": "mail.example.com"},
            "credentials": {"API_KEY": "hunter2-rotated", "REPORT_USERNAME": "hunter2-user"},
            "password_policy": {"min_length": 14},
            "accounts": [{"name": "sms", "auth_token": "hunter2-token"}],
            "webhook": "plain",
        }),
    )
    .await;
    set_settings(
        &tx,
        tenant,
        &json!({
            "gapi_key": "hunter2-gapi",
            "smtp": {"host": "mail.example.com"},
            "credentials": {"API_KEY": "hunter2-rotated", "REPORT_USERNAME": "hunter2-user"},
            "password_policy": {"min_length": 14},
            "accounts": [{"name": "sms", "auth_token": "hunter2-token"}],
            "webhook": {"url": "https://example.com", "clientSecret": "hunter2-client"},
        }),
    )
    .await;

    let found = entries(&tx, tenant).await;
    assert_eq!(found.len(), 3, "{found:#?}");
    for entry in &found {
        assert!(!entry.details.to_string().contains("hunter2"), "{entry:#?}");
        assert!(!entry.description.contains("hunter2"), "{entry:#?}");
    }

    let created = &found[0];
    assert_eq!(
        created.change("settings.gapi_key"),
        &json!({"path": "settings.gapi_key", "secret": true})
    );
    assert_eq!(
        created.change("settings.smtp.smtp_password"),
        &json!({"path": "settings.smtp.smtp_password", "secret": true})
    );
    // Everything under a secret is one redacted change.
    assert_eq!(
        created.change("settings.credentials"),
        &json!({"path": "settings.credentials", "secret": true})
    );
    assert_eq!(
        created.change("settings.password_policy.min_length"),
        &json!({"path": "settings.password_policy.min_length", "new": 12})
    );
    assert_eq!(
        created.change("settings.smtp.host"),
        &json!({"path": "settings.smtp.host", "new": "mail.example.com"})
    );

    assert_eq!(
        found[1].details["changes"],
        json!([
            {
                "path": "settings.accounts",
                "new": [{"name": "sms", "auth_token": {"secret": true}}],
            },
            {"path": "settings.credentials", "secret": true},
            {"path": "settings.password_policy.min_length", "old": 12, "new": 14},
            {"path": "settings.smtp.smtp_password", "secret": true},
            {"path": "settings.webhook", "new": "plain"},
        ])
    );
    assert_eq!(
        found[2].details["changes"],
        json!([{
            "path": "settings.webhook",
            "old": "plain",
            "new": {"url": "https://example.com", "clientSecret": {"secret": true}},
        }])
    );
}

#[tokio::test]
async fn the_redaction_list_names_secrets_and_spares_their_lookalikes() {
    let pool = schema::pool().await;
    let client = pool.get().await.unwrap();
    let is_secret = |path: &'static str| {
        let client = &client;
        async move {
            client
                .query_one("SELECT sequent_backend.audit_is_secret($1)", &[&path])
                .await
                .unwrap()
                .get::<_, bool>(0)
        }
    };

    for path in [
        "password",
        "settings.gapi_key",
        "settings.sms.auth_token",
        "settings.smtp.smtp_password",
        "settings.smtp.passwd",
        "settings.api_key",
        "settings.apikey",
        "settings.apiKey",
        "settings.APIKey",
        "settings.private_key",
        "settings.signing_key",
        "settings.client_secret",
        "settings.clientSecret",
        "settings.secret",
        "settings.token",
        "settings.credential",
        "settings.credentials.REPORT_USERNAME",
        "settings.twilio.account-auth-token",
        "credentials.ACCESS_TOKEN",
        "credentials.APP_SECRET",
        "credentials.VERIFY_TOKEN",
        "credentials.API_KEY",
        "credentials.SMTP_PASSWORD",
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "REPORT_PASSWORD",
        "settings.s3.access_key",
        "settings.s3.secret_key",
        "settings.password_hash",
        "settings.headers.Authorization",
    ] {
        assert!(is_secret(path).await, "{path} is a secret");
    }
    for path in [
        "settings",
        "settings.logs_timezone",
        "settings.gapi_email",
        "settings.password_policy",
        "settings.password_policy.min_length",
        "settings.passwordLength",
        "settings.token_verification_url",
        "settings.tokenUrl",
        "settings.accessTokenLifespan",
        "settings.secret_question",
        "settings.recaptcha_site_key",
        "settings.recaptchaSiteKey",
        "public_key",
        "share_encryption_public_key",
        "settings.keyboard",
        "settings.key",
        "REPORT_USERNAME",
        "settings.tokenizer",
        "presentation.i18n.en.name",
    ] {
        assert!(!is_secret(path).await, "{path} is not a secret");
    }
}

#[tokio::test]
async fn a_long_text_is_logged_as_its_sha256() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = insert_tenant(&tx, &json!({})).await;
    let longest_kept = "a".repeat(MAX_VALUE_BYTES);
    let too_long = "b".repeat(MAX_VALUE_BYTES + 1);
    // Two bytes each: half the limit in characters is already past it.
    let multibyte = "ñ".repeat(MAX_VALUE_BYTES / 2 + 1);
    let template = insert_template(&tx, tenant, "<p>Short</p>").await;

    act_as(&tx, ADMIN_USER, tenant).await;
    set_template_body(&tx, template, &longest_kept).await;
    set_template_body(&tx, template, &too_long).await;
    set_template_body(&tx, template, &multibyte).await;
    set_template_body(&tx, template, "<p>Short again</p>").await;

    let found = entries(&tx, tenant).await;
    assert_eq!(found.len(), 4, "{} entries", found.len());
    let path = "template.email.html_body";
    for entry in &found {
        assert_eq!(entry.election_event_id, None);
        assert_eq!(
            entry.description,
            format!("Updated template \"Receipt\": {path}")
        );
        assert_eq!(entry.paths(), [path]);
    }
    assert_eq!(
        found[0].change(path),
        &json!({"path": path, "old": "<p>Short</p>", "new": longest_kept})
    );
    assert_eq!(
        found[1].change(path),
        &json!({
            "path": path,
            "old_sha256": sha256_hex(&longest_kept),
            "new_sha256": sha256_hex(&too_long),
        })
    );
    assert_eq!(
        found[2].change(path),
        &json!({
            "path": path,
            "old_sha256": sha256_hex(&too_long),
            "new_sha256": sha256_hex(&multibyte),
        })
    );
    assert_eq!(
        found[3].change(path),
        &json!({
            "path": path,
            "old_sha256": sha256_hex(&multibyte),
            "new_sha256": sha256_hex("<p>Short again</p>"),
        })
    );
}

#[tokio::test]
async fn a_description_lists_five_paths_and_counts_the_rest() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = insert_tenant(&tx, &json!({})).await;

    act_as(&tx, ADMIN_USER, tenant).await;
    set_settings(
        &tx,
        tenant,
        &json!({"a": 1, "b": 1, "c": 1, "d": 1, "e": 1}),
    )
    .await;
    set_settings(
        &tx,
        tenant,
        &json!({"a": 2, "b": 2, "c": 2, "d": 2, "e": 2, "f": 2, "g": 2}),
    )
    .await;

    let found = entries(&tx, tenant).await;
    assert_eq!(found.len(), 2, "{found:#?}");
    let subject = format!("Updated tenant \"{}\"", slug(tenant));
    assert_eq!(
        found[0].description,
        format!("{subject}: settings.a, settings.b, settings.c, settings.d, settings.e")
    );
    assert_eq!(
        found[1].description,
        format!("{subject}: settings.a, settings.b, settings.c, settings.d, settings.e and 2 more")
    );
    assert_eq!(found[1].changes().len(), 7);
}

#[tokio::test]
async fn a_row_without_a_name_or_a_tenant_uses_its_id_and_the_sessions_tenant() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let tenant = insert_tenant(&tx, &json!({})).await;

    act_as(&tx, ADMIN_USER, tenant).await;
    let trustee: Uuid = tx
        .query_one(
            "INSERT INTO sequent_backend.trustee (public_key) VALUES ('key') RETURNING id",
            &[],
        )
        .await
        .unwrap()
        .get(0);

    let entry = one_entry(&tx, tenant).await;
    assert_eq!(entry.election_event_id, None);
    assert_eq!(entry.description, format!("Created trustee {trustee}"));
    assert_eq!(
        entry.details["object"],
        json!({"type": "trustee", "id": trustee, "name": null})
    );
}

#[tokio::test]
async fn the_outbox_refuses_kinds_the_worker_cannot_post() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let insert = "INSERT INTO sequent_backend.audit_log_outbox
            (tenant_id, statement_kind, event_type, log_type, attempts, description, details)
        VALUES ($1, $2, $3, $4, $5, 'Started', '{}')";
    for (kind, event_type, log_type, attempts, accepted) in [
        ("LogWriterStarted", "SYSTEM", "INFO", 0, true),
        ("KeycloakAdminEvent", "USER", "ERROR", 3, true),
        ("CastVote", "USER", "INFO", 0, false),
        ("AdminChange", "ADMIN", "INFO", 0, false),
        ("AdminChange", "USER", "WARN", 0, false),
        ("AdminChange", "USER", "INFO", -1, false),
    ] {
        let tx = client.transaction().await.unwrap();
        let result = tx
            .execute(
                insert,
                &[&Uuid::new_v4(), &kind, &event_type, &log_type, &attempts],
            )
            .await;
        assert_eq!(
            result.is_ok(),
            accepted,
            "{kind} {event_type} {log_type} {attempts}"
        );
    }
}
