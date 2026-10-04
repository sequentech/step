// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

/// A lockdown change travels as an outbox entry: a `Signing` body whose
/// kind sets the head, so it is posted once and in order with the signing
/// steps.
#[test]
fn a_lockdown_entry_sets_its_own_kind() {
    let body = StatementBody::Signing(SigningLogEntry {
        kind: SigningStatementKind::LockdownChanged,
        event_type: StatementEventType::SYSTEM,
        log_type: StatementLogType::INFO,
        description: "Locked down the election event on schedule.".to_string(),
        details_json: "{}".to_string(),
        step_id: "step".to_string(),
    });
    let head = StatementHead::from_body(EventIdString("event".to_string()), &body);
    assert!(matches!(head.kind, StatementType::LockdownChanged));
    assert_eq!(
        head.description,
        "Locked down the election event on schedule."
    );
    assert_eq!(
        SigningStatementKind::LockdownChanged
            .statement_type()
            .to_string(),
        "LockdownChanged"
    );
}

#[test]
fn the_lockdown_kinds_are_appended() {
    assert_eq!(
        borsh::to_vec(&StatementType::LockdownChanged).unwrap(),
        vec![51]
    );
    assert_eq!(
        borsh::to_vec(&SigningStatementKind::LockdownChanged).unwrap(),
        vec![21]
    );
}
