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
    /// Receipts are downloaded through the authenticated ownership action.
    pub(super) fn for_report(
        _report_type: &ReportType,
        _contains_voter_secrets: bool,
        _is_real: bool,
    ) -> Self {
        Self::default()
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
            ReportType::ACTIVITY_LOGS,
            ReportType::PARTICIPATION_REPORT,
            ReportType::INITIALIZATION_REPORT,
            ReportType::ELECTORAL_RESULTS,
            ReportType::BALLOT_IMAGES,
            ReportType::CREDENTIALS,
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

    /// Real receipts remain private even without declared secret attributes.
    #[test]
    fn ballot_receipts_use_authenticated_private_downloads() {
        assert_eq!(
            DocumentVisibility::for_report(&ReportType::BALLOT_RECEIPT, false, true),
            DocumentVisibility::Private
        );
        assert!(
            !DocumentVisibility::for_report(&ReportType::BALLOT_RECEIPT, false, true).is_public()
        );
        assert_eq!(
            DocumentVisibility::for_report(&ReportType::BALLOT_RECEIPT, true, true),
            DocumentVisibility::Private
        );
    }
}
