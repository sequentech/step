// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(non_camel_case_types)]

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::{Display, EnumString};

use crate::types::tally_sheets::{AreaContestResults, VotingChannel};

#[derive(
    Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Display, EnumString,
)]
pub enum TallySheetImportSourceFormat {
    ESS_ENHANCED_XML,
    CANONICAL_CSV,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Display, EnumString,
)]
pub enum TallySheetImportChangeType {
    NEW,
    CHANGED,
    UNCHANGED,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Display, EnumString,
)]
pub enum TallySheetImportStatus {
    PENDING_REVIEW,
    APPROVED,
    DISAPPROVED,
    FAILED_VALIDATION,
    CONFLICTED,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Display, EnumString,
)]
pub enum TallySheetImportItemStatus {
    PENDING_REVIEW,
    APPROVED,
    DISAPPROVED,
    CONFLICTED,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Display, EnumString,
)]
pub enum TallySheetImportReviewDecision {
    APPROVE,
    DISAPPROVE,
}

/// `message` is a pre-formatted English sentence kept for logs, CLI output,
/// and any other non-translated context. `code` + `params` are what a UI
/// should use to render a translated message (`t(code, params)`); `params`
/// is empty for structural/parsing errors that don't reduce to a fixed set
/// of translation keys (e.g. malformed CSV rows or XML).
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TallySheetImportValidationError {
    pub code: String,
    pub message: String,
    pub channel: Option<VotingChannel>,
    pub area_name: Option<String>,
    pub contest_external_id: Option<String>,
    pub candidate_external_id: Option<String>,
    pub field: Option<String>,
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct TallySheetImportSummary {
    pub imported_ballot_box_count: usize,
    pub changed_ballot_box_count: usize,
    pub new_ballot_box_count: usize,
    pub unchanged_ballot_box_count: usize,
    pub conflicted_ballot_box_count: usize,
    pub validation_error_count: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TallySheetImportPreviewItem {
    pub channel: VotingChannel,
    pub area_id: String,
    pub area_name: String,
    pub contest_id: String,
    pub contest_name: String,
    pub election_id: String,
    pub baseline_tally_sheet_id: Option<String>,
    pub baseline_version: Option<i32>,
    pub baseline_content_hash: Option<String>,
    pub previous: Option<AreaContestResults>,
    pub incoming: AreaContestResults,
    pub previous_csv: Option<String>,
    pub incoming_csv: String,
    pub incoming_content_hash: String,
    pub change_type: TallySheetImportChangeType,
    pub source_refs: Option<Value>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TallySheetImportPreview {
    pub document_id: String,
    pub source_format: TallySheetImportSourceFormat,
    pub selected_channel: VotingChannel,
    pub summary: TallySheetImportSummary,
    pub items: Vec<TallySheetImportPreviewItem>,
    pub validation_errors: Vec<TallySheetImportValidationError>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TallySheetImport {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub source_document_id: String,
    pub source_file_name: Option<String>,
    pub source_sha256: Option<String>,
    pub source_format: TallySheetImportSourceFormat,
    pub selected_channel: VotingChannel,
    pub status: TallySheetImportStatus,
    pub created_by_user_id: String,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub summary: TallySheetImportSummary,
    pub validation_report: Option<Value>,
    pub canonical_csv_sha256: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TallySheetImportItem {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: String,
    pub import_id: String,
    pub election_id: String,
    pub area_id: String,
    pub contest_id: String,
    pub channel: VotingChannel,
    pub generated_tally_sheet_id: Option<String>,
    pub baseline_approved_tally_sheet_id: Option<String>,
    pub baseline_approved_version: Option<i32>,
    pub baseline_content_hash: Option<String>,
    pub incoming_content_hash: String,
    pub change_type: TallySheetImportChangeType,
    pub status: TallySheetImportItemStatus,
    pub previous_csv: Option<String>,
    pub incoming_csv: String,
    pub source_refs: Option<Value>,
    pub validation_warnings: Option<Value>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
}
