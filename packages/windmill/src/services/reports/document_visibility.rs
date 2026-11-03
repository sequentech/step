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
    /// Public storage is reserved for actual voter receipts without declared secrets.
    pub(super) fn for_report(
        report_type: &ReportType,
        contains_voter_secrets: bool,
        is_real: bool,
    ) -> Self {
        match (report_type, contains_voter_secrets, is_real) {
            (ReportType::BALLOT_RECEIPT, false, true) => Self::Public,
            _ => Self::default(),
        }
    }

    /// Convert the visibility policy into the existing document-storage flag.
    pub(super) fn is_public(self) -> bool {
        matches!(self, Self::Public)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Non-receipt reports remain private regardless of secret-attribute declarations.
    #[test]
    fn reports_are_private_by_default_including_manual_verification() {
        assert_eq!(DocumentVisibility::default(), DocumentVisibility::Private);
        for report_type in [
            ReportType::MANUAL_VERIFICATION,
            ReportType::VOTE_RECEIPT,
            ReportType::ELECTORAL_RESULTS,
            ReportType::STATISTICAL_REPORT,
            ReportType::ACTIVITY_LOGS,
            ReportType::TRANSMISSION_REPORT,
            ReportType::STATUS,
            ReportType::OV_PRE_ENROLLED_APPROVED,
            ReportType::OV_WHO_VOTED,
            ReportType::PRE_ENROLLED_OV_SUBJECT_TO_MANUAL_VALIDATION,
            ReportType::PRE_ENROLLED_OV_BUT_DISAPPROVED,
            ReportType::LIST_OF_OVERSEAS_VOTERS,
            ReportType::OVCS_STATISTICS,
            ReportType::OVCS_INFORMATION,
            ReportType::OVCS_EVENTS,
            ReportType::OV_WITH_VOTING_STATUS,
            ReportType::INITIALIZATION_REPORT,
            ReportType::AUDIT_LOGS,
            ReportType::OV_NOT_YET_PRE_ENROLLED_LIST,
            ReportType::OV_NOT_YET_PRE_ENROLLED_NUMBER,
            ReportType::OV_TURNOUT_PERCENTAGE,
            ReportType::OV_TURNOUT_PER_ABOARD_STATUS_SEX,
            ReportType::OV_TURNOUT_PER_ABOARD_STATUS_SEX_PERCENTAGE,
            ReportType::BALLOT_IMAGES,
        ] {
            assert_eq!(
                DocumentVisibility::for_report(&report_type, false, true),
                DocumentVisibility::Private
            );
            assert!(!DocumentVisibility::for_report(&report_type, true, true).is_public());
        }
    }

    /// Administrative previews never use public receipt storage.
    #[test]
    fn ballot_receipt_previews_are_private() {
        for contains_voter_secrets in [false, true] {
            assert_eq!(
                DocumentVisibility::for_report(
                    &ReportType::BALLOT_RECEIPT,
                    contains_voter_secrets,
                    false
                ),
                DocumentVisibility::Private
            );
        }
    }

    /// Actual voter receipts preserve public downloads only when they contain no declared secrets.
    #[test]
    fn ballot_receipts_are_public_only_without_voter_secret_attributes() {
        assert_eq!(
            DocumentVisibility::for_report(&ReportType::BALLOT_RECEIPT, false, true),
            DocumentVisibility::Public
        );
        assert!(
            DocumentVisibility::for_report(&ReportType::BALLOT_RECEIPT, false, true).is_public()
        );
        assert_eq!(
            DocumentVisibility::for_report(&ReportType::BALLOT_RECEIPT, true, true),
            DocumentVisibility::Private
        );
    }
}
