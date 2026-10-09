// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::ReportType;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum DocumentVisibility {
    #[default]
    Private,
    Public,
}

impl DocumentVisibility {
    pub(super) fn for_report(report_type: &ReportType, contains_voter_secrets: bool) -> Self {
        match (report_type, contains_voter_secrets) {
            (ReportType::BALLOT_RECEIPT, false) => Self::Public,
            _ => Self::default(),
        }
    }

    pub(super) fn is_public(self) -> bool {
        matches!(self, Self::Public)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_are_private_by_default_including_manual_verification() {
        assert_eq!(DocumentVisibility::default(), DocumentVisibility::Private);
        for report_type in [
            ReportType::MANUAL_VERIFICATION,
            ReportType::ACTIVITY_LOGS,
            ReportType::PARTICIPATION_REPORT,
            ReportType::INITIALIZATION_REPORT,
            ReportType::ELECTORAL_RESULTS,
            ReportType::BALLOT_IMAGES,
            ReportType::CREDENTIALS,
        ] {
            assert_eq!(
                DocumentVisibility::for_report(&report_type, false),
                DocumentVisibility::Private
            );
            assert!(!DocumentVisibility::for_report(&report_type, true).is_public());
        }
    }

    #[test]
    fn ballot_receipts_are_public_only_without_voter_secret_attributes() {
        assert_eq!(
            DocumentVisibility::for_report(&ReportType::BALLOT_RECEIPT, false),
            DocumentVisibility::Public
        );
        assert!(DocumentVisibility::for_report(&ReportType::BALLOT_RECEIPT, false).is_public());
        assert_eq!(
            DocumentVisibility::for_report(&ReportType::BALLOT_RECEIPT, true),
            DocumentVisibility::Private
        );
    }
}
