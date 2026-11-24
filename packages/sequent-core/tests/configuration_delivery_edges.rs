// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(feature = "election_config_signing")]
#[test]
fn oversized_envelope_is_rejected_before_any_password_work() {
    use sequent_core::election_config::encrypted_delivery::{
        decrypt_signed_delivery, EncryptedDeliveryError, MAX_ENCRYPTED_SIGNED_DELIVERY_BYTES,
    };

    // A zero-initialized buffer is normally demand paged. The receiver must refuse the length
    // without reading the payload, copying it or deriving a key.
    let oversized = vec![0_u8; MAX_ENCRYPTED_SIGNED_DELIVERY_BYTES + 1];
    let error = decrypt_signed_delivery(&oversized, "").unwrap_err();
    assert_eq!(error, EncryptedDeliveryError::TooLarge);
    assert!(error.to_string().contains("256 MiB"));
    let malformed = decrypt_signed_delivery(&oversized[..12], "").unwrap_err();
    assert_eq!(malformed, EncryptedDeliveryError::InvalidEnvelope);
    assert!(malformed.to_string().contains("not a supported encrypted"));
}

#[cfg(feature = "election_config_templates")]
mod workbook {
    use sequent_core::election_config::{
        paths::Cell,
        plan_from_workbook::plan_from_workbook,
        problem::Code,
        sheet::{Sheet, Workbook},
    };

    fn grid(name: &str, rows: &[&[&str]]) -> Sheet {
        let cells = rows
            .iter()
            .map(|row| row.iter().map(|v| Cell::text(*v)).collect())
            .collect::<Vec<Vec<_>>>();
        Sheet::from_grid(name, &cells).unwrap()
    }

    fn workbook(schedule: &[&[&str]]) -> Workbook {
        Workbook::new(vec![
            grid("ElectionEvent", &[&["external_id"], &["demo-event"]]),
            grid("Elections", &[&["external_id"], &["north"], &["south"]]),
            grid("ScheduledEvents", schedule),
        ])
        .unwrap()
    }

    #[test]
    fn importing_a_subset_schedule_does_not_open_the_unspecified_election() {
        let result = plan_from_workbook(&workbook(&[
            &["event_type", "scheduled_datetime", "election.external_id"],
            &["START_VOTING_PERIOD", "2027-03-01T16:00:00+08:00", "north"],
        ]))
        .unwrap();
        assert_eq!(result.plan.elections.len(), 2);
        assert!(result.plan.schedule.voting_opens.is_none());
        assert!(result.plan.schedule.voting_closes.is_none());
        let rows = &result
            .plan
            .platform
            .iter()
            .find(|sheet| sheet.key == "scheduledevents")
            .unwrap()
            .rows;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].text("election.external_id"), Some("north"));
        assert_eq!(
            rows[0].text("scheduled_datetime"),
            Some("2027-03-01T16:00:00+08:00")
        );
    }

    #[test]
    fn importing_two_event_wide_starts_does_not_choose_one_silently() {
        let error = plan_from_workbook(&workbook(&[
            &["event_type", "scheduled_datetime"],
            &["START_VOTING_PERIOD", "2027-03-01T16:00:00Z"],
            &["START_VOTING_PERIOD", "2027-03-02T16:00:00Z"],
        ]))
        .err()
        .expect("duplicate whole-event start must be refused");
        assert!(error.errors().any(|p| p.code == Code::DuplicateId));
    }
}
