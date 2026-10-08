// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

fn entry(
    kind: SigningStatementKind,
    event_type: StatementEventType,
    log_type: StatementLogType,
) -> SigningLogEntry {
    SigningLogEntry {
        kind,
        event_type,
        log_type,
        description: "Signature refused on 7F3A-91C2: trusted-issuer".to_string(),
        details_json: r#"{"code":"7F3A-91C2"}"#.to_string(),
        step_id: "2b7c9e40-1f5d-4a8e-9c3b-6d2e1f0a9b87".to_string(),
    }
}

fn head(entry: SigningLogEntry) -> StatementHead {
    let event = EventIdString("event-id".to_string());
    StatementHead::from_body(event.clone(), &StatementBody::Signing(entry))
}

/// Signing is the only body whose head the caller sets: a step writes a
/// USER and a SYSTEM entry of the same kind, and a refusal is an ERROR.
#[test]
fn the_head_comes_from_the_entry() {
    let user = head(entry(
        SigningStatementKind::SigningSignatureRefused,
        StatementEventType::USER,
        StatementLogType::INFO,
    ));
    assert!(matches!(user.kind, StatementType::SigningSignatureRefused));
    assert!(matches!(user.event_type, StatementEventType::USER));
    assert!(matches!(user.log_type, StatementLogType::INFO));
    assert_eq!(
        user.description,
        "Signature refused on 7F3A-91C2: trusted-issuer"
    );

    let system = head(entry(
        SigningStatementKind::SigningSignatureRefused,
        StatementEventType::SYSTEM,
        StatementLogType::ERROR,
    ));
    assert!(matches!(
        system.kind,
        StatementType::SigningSignatureRefused
    ));
    assert!(matches!(system.event_type, StatementEventType::SYSTEM));
    assert!(matches!(system.log_type, StatementLogType::ERROR));
}

#[test]
fn every_signing_kind_names_its_own_statement_type() {
    let kinds = [
        SigningStatementKind::SigningRequestCreated,
        SigningStatementKind::SigningCertificateOpenFailed,
        SigningStatementKind::SigningRequestSigned,
        SigningStatementKind::SigningSignatureRefused,
        SigningStatementKind::SigningCertificateRegistered,
        SigningStatementKind::SigningHandover,
        SigningStatementKind::SigningRequestCancelled,
        SigningStatementKind::SigningRequestExpired,
        SigningStatementKind::SigningRequestCompleted,
        SigningStatementKind::SigningActionExecuted,
        SigningStatementKind::SigningRuleChanged,
        SigningStatementKind::SigningPermissionChanged,
        SigningStatementKind::SigningIssuerChanged,
        SigningStatementKind::SigningChecksChanged,
        SigningStatementKind::SigningCertificateRevoked,
        SigningStatementKind::SigningRequestsExported,
        // Scheduling entries (VOTE-LIFECYCLE) share the outbox.
        SigningStatementKind::LifecycleWindowChanged,
        SigningStatementKind::ScheduleRecomputeApplied,
        SigningStatementKind::ScheduleImported,
        // VOTE-LIFECYCLE: steps of scheduled openings and closings.
        SigningStatementKind::ScheduledOutcomeChanged,
    ];
    for kind in kinds {
        let statement_kind = kind.statement_type().to_string();
        // The kind is stored in the board's `statement_kind` column.
        assert_eq!(statement_kind, kind.to_string());
        assert!(
            statement_kind.starts_with("Signing")
                || matches!(
                    kind,
                    SigningStatementKind::LifecycleWindowChanged
                        | SigningStatementKind::ScheduleRecomputeApplied
                        | SigningStatementKind::ScheduleImported
                        | SigningStatementKind::ScheduledOutcomeChanged
                ),
            "{statement_kind}"
        );
        assert!(statement_kind.len() <= 40, "{statement_kind}");
    }
}

#[test]
fn the_signing_body_and_kinds_are_appended() {
    let body = StatementBody::Signing(entry(
        SigningStatementKind::SigningRequestCreated,
        StatementEventType::USER,
        StatementLogType::INFO,
    ));
    assert_eq!(borsh::to_vec(&body).unwrap()[0], 30);

    let appended = [
        (StatementType::SigningRequestCreated, 30),
        (StatementType::SigningCertificateOpenFailed, 31),
        (StatementType::SigningRequestSigned, 32),
        (StatementType::SigningSignatureRefused, 33),
        (StatementType::SigningCertificateRegistered, 34),
        (StatementType::SigningHandover, 35),
        (StatementType::SigningRequestCancelled, 36),
        (StatementType::SigningRequestExpired, 37),
        (StatementType::SigningRequestCompleted, 38),
        (StatementType::SigningActionExecuted, 39),
        (StatementType::SigningRuleChanged, 40),
        (StatementType::SigningPermissionChanged, 41),
        (StatementType::SigningIssuerChanged, 42),
        (StatementType::SigningChecksChanged, 43),
        (StatementType::SigningCertificateRevoked, 44),
        (StatementType::SigningRequestsExported, 45),
        (StatementType::LifecycleWindowChanged, 46),
        (StatementType::ScheduleRecomputeApplied, 47),
        (StatementType::ScheduleImported, 48),
        (StatementType::ScheduledOutcomeChanged, 49),
    ];
    for (kind, discriminant) in appended {
        assert_eq!(borsh::to_vec(&kind).unwrap(), vec![discriminant], "{kind}");
    }
}

/// The entry's fields are all signed: the step id the worker dedupes on
/// and the details the Logs tab shows.
#[test]
fn the_entry_round_trips_through_borsh() {
    let original = entry(
        SigningStatementKind::SigningRequestSigned,
        StatementEventType::SYSTEM,
        StatementLogType::INFO,
    );
    let bytes = borsh::to_vec(&StatementBody::Signing(original.clone())).unwrap();
    let decoded: StatementBody = borsh::from_slice(&bytes).unwrap();
    match decoded {
        StatementBody::Signing(decoded) => assert_eq!(decoded, original),
        other => panic!("decoded {other:?}"),
    }

    let other_step = SigningLogEntry {
        step_id: "0b7c9e40-1f5d-4a8e-9c3b-6d2e1f0a9b87".to_string(),
        ..original.clone()
    };
    assert_ne!(
        borsh::to_vec(&StatementBody::Signing(other_step)).unwrap(),
        bytes
    );
}
