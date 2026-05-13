// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Celery task identifiers and their human-readable display names.
use strum_macros::{Display, EnumString, EnumVariantNames};

/// Celery-backed long-running jobs Windmill registers by name.
#[derive(Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames)]
pub enum ETasksExecution {
    /// Export the full election event.
    EXPORT_ELECTION_EVENT,
    /// Export tenant-level configuration.
    EXPORT_TENANT_CONFIG,
    /// Restore tenant configuration from an export bundle.
    IMPORT_TENANT_CONFIG,
    /// Import candidates.
    IMPORT_CANDIDATES,
    /// Import voters.
    IMPORT_USERS,
    /// Create a new election event.
    CREATE_ELECTION_EVENT,
    /// Restore an election from an exported archive.
    IMPORT_ELECTION_EVENT,
    /// Export the voter register for an event.
    EXPORT_VOTERS,
    /// Build the transmission package for signing.
    CREATE_TRANSMISSION_PACKAGE,
    /// Export published ballot material for auditors.
    EXPORT_BALLOT_PUBLICATION,
    /// Generate the activity log spreadsheet or PDF report.
    EXPORT_ACTIVITY_LOGS_REPORT,
    /// Produce a voter-facing ballot receipt artifact.
    CREATE_BALLOT_RECEIPT,
    /// Run a configured report template against live data.
    GENERATE_REPORT,
    /// Materialize a document template with current placeholders.
    GENERATE_TEMPLATE,
    /// Emit the transmission audit report after Miru dispatch.
    GENERATE_TRANSMISSION_REPORT,
    /// Export voter enrollment applications for an event.
    EXPORT_APPLICATION,
    /// Import applications.
    IMPORT_APPLICATION,
    /// Export trustee keys and ceremony configuration.
    EXPORT_TRUSTEES,
    /// Render a document to PDF.
    RENDER_DOCUMENT_PDF,
    /// Create a new tenant.
    CREATE_TENANT,
    /// Export templates.
    EXPORT_TEMPLATES,
    /// Import templates.
    IMPORT_TEMPLATES,
    /// Delete an election event.
    DELETE_ELECTION_EVENT,
    /// Prepare publication preview assets.
    PREPARE_PUBLICATION_PREVIEW,
    /// Spreadsheet export of decoded tally results.
    EXPORT_TALLY_RESULTS_XLSX,
    /// Export certificate authorities.
    EXPORT_CERTIFICATE_AUTHORITIES,
}

impl ETasksExecution {
    /// Human-readable label shown in admin UIs and logs for this task.
    #[must_use]
    pub const fn to_name(&self) -> &str {
        match self {
            ETasksExecution::EXPORT_ELECTION_EVENT => "Export Election Event",
            ETasksExecution::EXPORT_TENANT_CONFIG => "Export Tenant Config",
            ETasksExecution::IMPORT_TENANT_CONFIG => "Import Tenant Config",
            ETasksExecution::IMPORT_CANDIDATES => "Import Candidates",
            ETasksExecution::IMPORT_USERS => "Import Voters",
            ETasksExecution::CREATE_ELECTION_EVENT => "Create Election Event",
            ETasksExecution::IMPORT_ELECTION_EVENT => "Import Election Event",
            ETasksExecution::EXPORT_VOTERS => "Export Voters",
            ETasksExecution::CREATE_TRANSMISSION_PACKAGE => "Create Transmission Package",
            ETasksExecution::EXPORT_BALLOT_PUBLICATION => "Export Ballot Publication",
            ETasksExecution::EXPORT_ACTIVITY_LOGS_REPORT => "Export Activity Logs Report",
            ETasksExecution::CREATE_BALLOT_RECEIPT => "Create Ballot Receipt",
            ETasksExecution::GENERATE_REPORT => "Generate Report",
            ETasksExecution::GENERATE_TEMPLATE => "Generate Template",
            ETasksExecution::GENERATE_TRANSMISSION_REPORT => "Generate Transmission Report",
            ETasksExecution::EXPORT_APPLICATION => "Export Application",
            ETasksExecution::IMPORT_APPLICATION => "Import Application",
            ETasksExecution::EXPORT_TRUSTEES => "Export Trustees",
            ETasksExecution::RENDER_DOCUMENT_PDF => "Render Document PDF",
            ETasksExecution::CREATE_TENANT => "Create Tenant",
            ETasksExecution::EXPORT_TEMPLATES => "Export Templates",
            ETasksExecution::IMPORT_TEMPLATES => "Import Templates",
            ETasksExecution::DELETE_ELECTION_EVENT => "Delete Election Event",
            ETasksExecution::PREPARE_PUBLICATION_PREVIEW => "Prepare Publication Preview",
            ETasksExecution::EXPORT_TALLY_RESULTS_XLSX => "Export Tally Results To XLSX",
            ETasksExecution::EXPORT_CERTIFICATE_AUTHORITIES => "Export Certificate Authorities",
        }
    }
}
