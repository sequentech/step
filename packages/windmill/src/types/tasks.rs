// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use strum_macros::{Display, EnumString, EnumVariantNames};

#[derive(Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames)]
pub enum ETasksExecution {
    ImportCandidates,
    ImportUsers,
    ImportElectionEvent,
    ExportElectionEvent,
    ExportVoters,
    ExportActivityLogsReport,
    CreateTransmissionPackage,
}

impl ETasksExecution {
    pub fn to_name(&self) -> &str {
        match self {
            ETasksExecution::ImportCandidates => "Import Candidates",
            ETasksExecution::ImportUsers => "Import Voters",
            ETasksExecution::ImportElectionEvent => "Import Election Event",
            ETasksExecution::ExportVoters => "Export Voters",
            ETasksExecution::ExportElectionEvent => "Export Election Event",
            ETasksExecution::ExportActivityLogsReport => {
                "Export Election Event Activity Logs Report"
            }
            ETasksExecution::CreateTransmissionPackage => "Create Transmission Package",
        }
    }
}
