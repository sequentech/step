// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

/// An initialization travels as an outbox entry: a `Signing` body whose
/// kind sets the head, so it is posted once and in order with the
/// signing steps.
#[test]
fn an_initialization_entry_sets_its_own_kind() {
    let body = StatementBody::Signing(SigningLogEntry {
        kind: SigningStatementKind::ElectionInitialized,
        event_type: StatementEventType::SYSTEM,
        log_type: StatementLogType::INFO,
        description: "Initialized Post P, country C.".to_string(),
        details_json: "{}".to_string(),
        step_id: "step".to_string(),
    });
    let head = StatementHead::from_body(EventIdString("event".to_string()), &body);
    assert!(matches!(head.kind, StatementType::ElectionInitialized));
    assert_eq!(head.description, "Initialized Post P, country C.");
    assert_eq!(
        SigningStatementKind::ElectionInitialized
            .statement_type()
            .to_string(),
        "ElectionInitialized"
    );
}

#[test]
fn the_initialization_kinds_are_appended() {
    assert_eq!(
        borsh::to_vec(&StatementType::ElectionInitialized).unwrap(),
        vec![50]
    );
    assert_eq!(
        borsh::to_vec(&SigningStatementKind::ElectionInitialized).unwrap(),
        vec![20]
    );
}
