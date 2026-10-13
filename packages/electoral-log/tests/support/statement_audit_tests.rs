// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

fn entry(kind: AuditStatementKind) -> AuditLogEntry {
    AuditLogEntry {
        kind,
        event_type: StatementEventType::USER,
        log_type: StatementLogType::INFO,
        description: "Updated tenant \"acme\": settings.logs_timezone".to_string(),
        details_json: r#"{"action":"update"}"#.to_string(),
        entry_id: "0d1b2c3d-4e5f-4071-8293-a4b5c6d7e8f9".to_string(),
    }
}

/// An audit entry travels through the outbox: its kind, event type, log type
/// and description set the head, so the signed statement says who changed what.
#[test]
fn an_audit_entry_sets_its_own_head() {
    let body = StatementBody::Audit(AuditLogEntry {
        event_type: StatementEventType::SYSTEM,
        log_type: StatementLogType::ERROR,
        ..entry(AuditStatementKind::KeycloakAdminEvent)
    });
    let head = StatementHead::from_body(EventIdString("tenant".to_string()), &body);
    assert!(matches!(head.kind, StatementType::KeycloakAdminEvent));
    assert_eq!(head.event_type, StatementEventType::SYSTEM);
    assert_eq!(head.log_type, StatementLogType::ERROR);
    assert_eq!(
        head.description,
        "Updated tenant \"acme\": settings.logs_timezone"
    );
}

#[test]
fn the_audit_kinds_are_appended() {
    let expected = [
        (AuditStatementKind::AdminChange, 0u8, 62u8, "AdminChange"),
        (
            AuditStatementKind::KeycloakAdminEvent,
            1,
            63,
            "KeycloakAdminEvent",
        ),
        (AuditStatementKind::LogsExported, 2, 64, "LogsExported"),
        (
            AuditStatementKind::LogWriterStarted,
            3,
            65,
            "LogWriterStarted",
        ),
        (
            AuditStatementKind::LogWriterStopped,
            4,
            66,
            "LogWriterStopped",
        ),
    ];
    for (kind, kind_tag, type_tag, name) in expected {
        assert_eq!(borsh::to_vec(&kind).unwrap(), vec![kind_tag]);
        assert_eq!(
            borsh::to_vec(&kind.statement_type()).unwrap(),
            vec![type_tag]
        );
        assert_eq!(kind.to_string(), name);
        assert_eq!(kind.statement_type().to_string(), name);
        assert_eq!(name.parse::<AuditStatementKind>().unwrap(), kind);
    }
    assert!("CastVote".parse::<AuditStatementKind>().is_err());
    assert!(borsh::from_slice::<StatementType>(&[67]).is_err());
}

#[test]
fn the_audit_body_is_appended_and_round_trips() {
    let body = StatementBody::Audit(entry(AuditStatementKind::AdminChange));
    let bytes = borsh::to_vec(&body).unwrap();
    assert_eq!(bytes[0], 43);
    let StatementBody::Audit(decoded) = borsh::from_slice::<StatementBody>(&bytes).unwrap() else {
        panic!("not an audit entry");
    };
    assert_eq!(decoded, entry(AuditStatementKind::AdminChange));
}

fn keycloak_head(event_type: &str, text: &str) -> StatementHead {
    StatementHead::from_body(
        EventIdString("tenant".to_string()),
        &StatementBody::KeycloakUserEvent(
            ErrorMessageString(text.to_string()),
            KeycloakEventTypeString(event_type.to_string()),
        ),
    )
}

/// The tenant realm's events carry their details as JSON: the description
/// names the error, not the JSON, and an error makes an ERROR entry whatever
/// the event type. A lockout is an ERROR entry too.
#[test]
fn a_keycloak_event_with_json_details_reads_its_error_from_them() {
    let failed = keycloak_head(
        "LOGIN_ERROR",
        r#"{"error":"invalid_user_credentials","client_id":"admin-portal"}"#,
    );
    assert_eq!(failed.log_type, StatementLogType::ERROR);
    assert_eq!(failed.description, "LOGIN_ERROR invalid_user_credentials");

    let signed_in = keycloak_head("LOGIN", r#"{"error":null,"client_id":"admin-portal"}"#);
    assert_eq!(signed_in.log_type, StatementLogType::INFO);
    assert_eq!(signed_in.description, "LOGIN");

    let unplaced = keycloak_head("LOGIN", r#"{"error":"event_not_placed"}"#);
    assert_eq!(unplaced.log_type, StatementLogType::ERROR);
    assert_eq!(unplaced.description, "LOGIN event_not_placed");

    let locked = keycloak_head("USER_DISABLED_BY_TEMPORARY_LOCKOUT", r#"{"error":null}"#);
    assert_eq!(locked.log_type, StatementLogType::ERROR);
    assert_eq!(locked.description, "USER_DISABLED_BY_TEMPORARY_LOCKOUT");
}

/// Events with a plain error text keep their description.
#[test]
fn a_keycloak_event_with_a_plain_text_keeps_its_first_word() {
    let failed = keycloak_head("LOGIN_ERROR", "user_not_found and more");
    assert_eq!(failed.log_type, StatementLogType::ERROR);
    assert_eq!(failed.description, "LOGIN_ERROR user_not_found");
    let signed_in = keycloak_head("LOGIN", "null");
    assert_eq!(signed_in.log_type, StatementLogType::INFO);
    assert_eq!(signed_in.description, "LOGIN");
}
