// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::ReportType;
use sequent_core::types::hasura::core::DocumentPurpose;

pub(super) fn report_document_purpose(
    report_type: &ReportType,
    is_real: bool,
) -> Option<DocumentPurpose> {
    (*report_type == ReportType::BALLOT_RECEIPT && is_real)
        .then_some(DocumentPurpose::BallotReceipt)
}

pub(super) fn report_document_name(report_type: &ReportType, is_real: bool, name: &str) -> String {
    // Legacy receipts are protected by their filename as well as their purpose.
    // Keep sample previews outside that namespace without relaxing either guard.
    if *report_type == ReportType::BALLOT_RECEIPT
        && !is_real
        && name.to_ascii_lowercase().starts_with("ballot_receipt_")
    {
        format!("preview_{name}")
    } else {
        name.to_string()
    }
}

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
    use sequent_core::types::hasura::core::{Document, DocumentAnnotations};

    #[test]
    fn receipt_preview_is_downloadable_without_weakening_real_receipt_protection() {
        for is_real in [false, true] {
            let name = report_document_name(
                &ReportType::BALLOT_RECEIPT,
                is_real,
                "ballot_receipt_event.pdf",
            );
            let annotations = DocumentAnnotations {
                purpose: report_document_purpose(&ReportType::BALLOT_RECEIPT, is_real),
                ..Default::default()
            };
            let document: Document = serde_json::from_value(serde_json::json!({
                "id":"document", "name":name, "annotations":annotations, "is_public":false
            }))
            .unwrap();
            assert_eq!(
                document.is_ballot_receipt(),
                is_real,
                "Only real receipts are refused by ordinary administrative document access"
            );
            assert_eq!(document.is_public, Some(false));
            assert_eq!(
                name,
                if is_real {
                    "ballot_receipt_event.pdf"
                } else {
                    "preview_ballot_receipt_event.pdf"
                }
            );
        }
        assert_eq!(
            report_document_name(
                &ReportType::PARTICIPATION_REPORT,
                false,
                "participation.pdf"
            ),
            "participation.pdf"
        );
        assert_eq!(
            report_document_purpose(&ReportType::PARTICIPATION_REPORT, false),
            None
        );
    }

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
