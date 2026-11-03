// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use serde_json::json;

fn target() -> AuditLogTarget {
    AuditLogTarget {
        tenant_id: Uuid::nil(),
        election_event_id: None,
    }
}

fn change(action: AdminAction, changes: Vec<FieldChange>) -> AdminChange {
    AdminChange::new(
        target(),
        AuditObject::new("tenant", Uuid::nil(), Some("acme")),
        action,
        AuditActor {
            user_id: Some("user".into()),
            username: Some("ada".into()),
            role: Some("admin-user".into()),
        },
        AuditOrigin::Harvest,
    )
    .with_changes(changes)
}

#[test]
fn secrets_are_told_by_their_path() {
    for path in [
        "password",
        "settings.gapi_key",
        "settings.sms.auth_token",
        "smtp.Password",
        "credentials",
        "client_secret",
        "clientSecret",
        "attributes.private_key",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_ACCESS_KEY_ID",
        "headers.Authorization",
        "password_hash",
        "VERIFY_TOKEN",
        "webhook_key",
    ] {
        assert!(is_secret_path(path), "{path}");
    }
    for path in [
        "settings.logs_timezone",
        "password_policy",
        "passwordLength",
        "settings.otp_token_length",
        "reset_password_allowed",
        "tokenUrl",
        "accessTokenLifespan",
        "secret_question",
        "recaptcha_site_key",
        "public_key",
        "key",
        "name",
    ] {
        assert!(!is_secret_path(path), "{path}");
    }
}

#[test]
fn objects_are_compared_key_by_key() {
    let old = json!({"logs_timezone": "UTC", "sms": {"provider": "a", "auth_token": "x"},
        "kept": [1, 2], "removed": true});
    let new = json!({"logs_timezone": "Asia/Manila", "sms": {"provider": "b", "auth_token": "y"},
        "kept": [1, 2], "added": {"deep": 1}});
    let changes = diff_values("settings", Some(&old), Some(&new));
    let paths: Vec<&str> = changes.iter().map(|change| change.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "settings.added.deep",
            "settings.logs_timezone",
            "settings.removed",
            "settings.sms.auth_token",
            "settings.sms.provider",
        ]
    );
    assert_eq!(changes[0].old, None);
    assert_eq!(changes[0].new, Some(json!(1)));
    assert_eq!(changes[1].old, Some(json!("UTC")));
    assert_eq!(changes[1].new, Some(json!("Asia/Manila")));
    assert_eq!(changes[2].new, None);
    // The secret says only that it changed.
    assert!(changes[3].secret);
    assert_eq!((&changes[3].old, &changes[3].new), (&None, &None));
    assert_eq!(
        serde_json::to_value(&changes[3]).unwrap(),
        json!({"path": "settings.sms.auth_token", "secret": true})
    );
}

#[test]
fn secrets_inside_a_value_logged_whole_are_blanked() {
    let accounts =
        json!([{"name": "sms", "auth_token": "t", "options": {"apiKey": "k", "region": "eu"}}]);
    let changes = diff_values("accounts", None, Some(&accounts));
    assert_eq!(
        changes[0].new,
        Some(json!([{
            "name": "sms",
            "auth_token": {"secret": true},
            "options": {"apiKey": {"secret": true}, "region": "eu"},
        }]))
    );
    let replaced = diff_values(
        "x",
        Some(&json!({"password": "p", "user": "u"})),
        Some(&json!(3)),
    );
    assert_eq!(
        replaced[0].old,
        Some(json!({"password": {"secret": true}, "user": "u"}))
    );
}

#[test]
fn equal_values_and_absent_ones_are_no_change() {
    let value = json!({"a": 1, "b": null});
    assert!(diff_values("", Some(&value), Some(&json!({"a": 1}))).is_empty());
    assert!(diff_values("x", None, Some(&Value::Null)).is_empty());
    let scalar = diff_values("name", Some(&json!("a")), None);
    assert_eq!(scalar.len(), 1);
    assert_eq!(scalar[0].path, "name");
    // An object replaced by a scalar is one change of the whole value.
    let replaced = diff_values("x", Some(&json!({"a": 1})), Some(&json!(3)));
    assert_eq!(replaced.len(), 1);
    assert_eq!(replaced[0].old, Some(json!({"a": 1})));
}

#[test]
fn long_texts_keep_only_their_hashes() {
    let long = "a".repeat(LONG_TEXT_BYTES + 1);
    let change = FieldChange::new("body", Some(&json!("short")), Some(&json!(long)));
    assert_eq!((&change.old, &change.new), (&None, &None));
    assert_eq!(
        change.old_sha256.as_deref(),
        Some(sha256_hex("short").as_str())
    );
    assert_eq!(
        change.new_sha256.as_deref(),
        Some(sha256_hex(&long).as_str())
    );
    let at_limit = "a".repeat(LONG_TEXT_BYTES);
    let kept = FieldChange::new("body", None, Some(&json!(at_limit)));
    assert!(kept.new.is_some() && kept.new_sha256.is_none());
    let created = FieldChange::new("body", None, Some(&json!(long)));
    assert_eq!(created.old_sha256, None);
    let list = FieldChange::new("body", Some(&json!([long])), None);
    assert!(list.old_sha256.is_some() && list.new_sha256.is_none());
}

#[test]
fn a_change_describes_itself() {
    let fields: Vec<FieldChange> = (0..7)
        .map(|n| FieldChange::new(&format!("settings.f{n}"), None, Some(&json!(n))))
        .collect();
    assert_eq!(
        change(AdminAction::Update, fields[..2].to_vec()).describe(),
        "Updated tenant \"acme\": settings.f0, settings.f1"
    );
    assert_eq!(
        change(AdminAction::Update, fields).describe(),
        "Updated tenant \"acme\": settings.f0, settings.f1, settings.f2, settings.f3, \
         settings.f4 and 2 more"
    );
    assert_eq!(
        change(AdminAction::Delete, vec![]).describe(),
        "Deleted tenant \"acme\""
    );
    let mut unnamed = change(AdminAction::ResetPassword, vec![]);
    unnamed.object = AuditObject::new("staff_account", "42", None);
    assert_eq!(unnamed.describe(), "Reset the password of staff account 42");
    assert_eq!(
        unnamed.with_description("Imported 3 templates").describe(),
        "Imported 3 templates"
    );
}

#[test]
fn a_change_becomes_an_admin_change_entry() {
    let staged = change(
        AdminAction::Update,
        diff_values("name", Some(&json!("a")), Some(&json!("b"))),
    )
    .with_extra(json!({"count": 2}));
    let entry = staged.entry().unwrap();
    assert_eq!(entry.statement_kind, AuditStatementKind::AdminChange);
    assert_eq!(entry.event_type, StatementEventType::USER);
    assert_eq!(entry.log_type, StatementLogType::INFO);
    assert_eq!(entry.user_id.as_deref(), Some("user"));
    assert_eq!(entry.username.as_deref(), Some("ada"));
    assert_eq!(
        entry.details,
        json!({
            "object": {"type": "tenant", "id": Uuid::nil().to_string(), "name": "acme"},
            "action": "update",
            "actor": {"user_id": "user", "username": "ada", "role": "admin-user"},
            "origin": "harvest",
            "changes": [{"path": "name", "old": "a", "new": "b"}],
            "extra": {"count": 2},
        })
    );
    // Without an actor it is the system's entry, and carries no `extra`.
    let mut system = change(AdminAction::Import, vec![]);
    system.actor = AuditActor::default();
    let entry = system.entry().unwrap();
    assert_eq!(entry.event_type, StatementEventType::SYSTEM);
    assert!(entry.details.get("extra").is_none());
    assert_eq!(AdminAction::ResetPassword.to_string(), "reset_password");
    assert_eq!(
        "windmill".parse::<AuditOrigin>().unwrap(),
        AuditOrigin::Windmill
    );
}
