// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use strum_macros::{Display, EnumString, EnumVariantNames};

#[derive(Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames)]
pub enum EDocuments {
    ELECTION_EVENT,
    VOTERS,
    ACTIVITY_LOGS,
    SCHEDULED_EVENTS,
    S3_FILES,
    REPORTS,
}

impl EDocuments {
    pub fn to_file_name(&self) -> &str {
        match self {
            EDocuments::ELECTION_EVENT => "export_election_event",
            EDocuments::VOTERS => "export_voters",
            EDocuments::ACTIVITY_LOGS => "export_activity_logs",
            EDocuments::SCHEDULED_EVENTS => "export_scheduled_events",
            EDocuments::S3_FILES => "export_S3_files",
            EDocuments::REPORTS => "export_reports",
        }
    }
}
