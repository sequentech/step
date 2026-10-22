// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

fn entry(
    kind: TenantActivityKind,
    event_type: StatementEventType,
    log_type: StatementLogType,
) -> TenantActivityEntry {
    TenantActivityEntry {
        tenant_id: TenantIdString("90505c8a-23a9-4cdf-a26b-4e19f6a097d5".to_string()),
        kind,
        event_type,
        log_type,
        description: "Deleted an election event".to_string(),
        details_json: r#"{"action":"delete-election-event"}"#.to_string(),
    }
}

/// As for a signing entry, the caller sets the head: a refused request is an
/// ERROR entry of the same kind as an accepted one.
#[test]
fn the_head_comes_from_the_entry() {
    let event = EventIdString("Generic Event".to_string());
    let accepted = StatementHead::from_body(
        event.clone(),
        &StatementBody::TenantActivity(entry(
            TenantActivityKind::TenantAdminAction,
            StatementEventType::USER,
            StatementLogType::INFO,
        )),
    );
    assert!(matches!(accepted.kind, StatementType::TenantAdminAction));
    assert_eq!(accepted.event_type, StatementEventType::USER);
    assert_eq!(accepted.log_type, StatementLogType::INFO);
    assert_eq!(accepted.description, "Deleted an election event");
    assert_eq!(accepted.event, event);

    let refused = StatementHead::from_body(
        event.clone(),
        &StatementBody::TenantActivity(entry(
            TenantActivityKind::TenantAdminAction,
            StatementEventType::USER,
            StatementLogType::ERROR,
        )),
    );
    assert!(matches!(refused.kind, StatementType::TenantAdminAction));
    assert_eq!(refused.log_type, StatementLogType::ERROR);

    let opened = StatementHead::from_body(
        event,
        &StatementBody::TenantActivity(entry(
            TenantActivityKind::TenantLogOpened,
            StatementEventType::SYSTEM,
            StatementLogType::INFO,
        )),
    );
    assert!(matches!(opened.kind, StatementType::TenantLogOpened));
    assert_eq!(opened.event_type, StatementEventType::SYSTEM);
}

#[test]
fn the_tenant_activity_body_and_kinds_are_appended() {
    let body = StatementBody::TenantActivity(entry(
        TenantActivityKind::TenantLogOpened,
        StatementEventType::SYSTEM,
        StatementLogType::INFO,
    ));
    assert_eq!(borsh::to_vec(&body).unwrap()[0], 43);

    for (kind, statement_type, index, discriminant) in [
        (
            TenantActivityKind::TenantLogOpened,
            StatementType::TenantLogOpened,
            0u8,
            62u8,
        ),
        (
            TenantActivityKind::TenantAdminAction,
            StatementType::TenantAdminAction,
            1,
            63,
        ),
    ] {
        assert_eq!(borsh::to_vec(&kind).unwrap(), vec![index], "{kind}");
        assert_eq!(
            borsh::to_vec(&statement_type).unwrap(),
            vec![discriminant],
            "{statement_type}"
        );
        // The statement type is named after its kind and fits the kind column.
        let statement_kind = kind.statement_type().to_string();
        assert_eq!(statement_kind, kind.to_string());
        assert!(statement_kind.len() <= 40, "{statement_kind}");
    }
}
