// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use strum_macros::{Display, EnumString, EnumVariantNames};

#[derive(Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames)]
pub enum ETasksExecution {
    EXPORT_ELECTION_EVENT,
    EXPORT_TENANT_CONFIG,
    IMPORT_TENANT_CONFIG,
    IMPORT_CANDIDATES,
    IMPORT_USERS,
    CREATE_ELECTION_EVENT,
    IMPORT_ELECTION_EVENT,
    EXPORT_VOTERS,
    CREATE_TRANSMISSION_PACKAGE,
    EXPORT_BALLOT_PUBLICATION,
    EXPORT_ACTIVITY_LOGS_REPORT,
    CREATE_BALLOT_RECEIPT,
    GENERATE_REPORT,
    GENERATE_TEMPLATE,
    GENERATE_TRANSMISSION_REPORT,
    EXPORT_APPLICATION,
    IMPORT_APPLICATION,
    EXPORT_TRUSTEES,
    RENDER_DOCUMENT_PDF,
    CREATE_TENANT,
    EXPORT_TEMPLATES,
    IMPORT_TEMPLATES,
    DELETE_ELECTION_EVENT,
    EDIT_USER,
    PREPARE_PUBLICATION_PREVIEW,
    EXPORT_TALLY_RESULTS_XLSX,
    EXPORT_CERTIFICATE_AUTHORITIES,
    PUBLISH_RESULTS_WEBSITE,
    GENERATE_RECONCILIATION_PATCHES,
    APPLY_RECONCILIATION_PATCH,
    GENERATE_BALLOT_PUBLICATION,
    VOTER_INFORMATION_LETTER,
}

impl ETasksExecution {
    pub fn to_name(&self) -> &str {
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
            ETasksExecution::EDIT_USER => "Edit Voter",
            ETasksExecution::PREPARE_PUBLICATION_PREVIEW => "Prepare Publication Preview",
            ETasksExecution::EXPORT_TALLY_RESULTS_XLSX => "Export Tally Results To XLSX",
            ETasksExecution::EXPORT_CERTIFICATE_AUTHORITIES => "Export Certificate Authorities",
            ETasksExecution::PUBLISH_RESULTS_WEBSITE => "Publish Results Website",
            ETasksExecution::GENERATE_RECONCILIATION_PATCHES => "Generate Reconciliation Patches",
            ETasksExecution::APPLY_RECONCILIATION_PATCH => "Apply Reconciliation Changes",
            ETasksExecution::GENERATE_BALLOT_PUBLICATION => "Generate Ballot Publication",
            ETasksExecution::VOTER_INFORMATION_LETTER => "Voter Information Letter",
        }
    }
}
