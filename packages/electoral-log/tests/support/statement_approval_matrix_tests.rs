// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

fn body() -> StatementBody {
    StatementBody::ApprovalMatrixUpdated(
        EventIdString("event-id".to_string()),
        ApprovalMatrixVersion(2),
        ApprovalMatrixDigestString("ab12".to_string()),
    )
}

#[test]
fn a_saved_version_names_its_number_and_digest() {
    let head = StatementHead::from_body(EventIdString("event-id".to_string()), &body());

    assert!(matches!(head.kind, StatementType::ApprovalMatrixUpdated));
    assert!(matches!(head.event_type, StatementEventType::USER));
    assert!(matches!(head.log_type, StatementLogType::INFO));
    assert_eq!(
        head.description,
        "Enrollment approval matrix version 2 saved (SHA-256 ab12)."
    );
}

#[test]
fn the_statement_is_appended_to_the_wire_format() {
    assert_eq!(borsh::to_vec(&body()).unwrap()[0], 34);
    assert_eq!(
        borsh::to_vec(&StatementType::ApprovalMatrixUpdated).unwrap(),
        vec![54]
    );
    let decoded: StatementBody = borsh::from_slice(&borsh::to_vec(&body()).unwrap()).unwrap();
    assert!(matches!(
        decoded,
        StatementBody::ApprovalMatrixUpdated(_, ApprovalMatrixVersion(2), _)
    ));
}
