// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
    BULLETIN_BOARDS,
    PROTOCOL_MANAGER_KEYS,
    TRUSTEES_CONFIGURATION,
    TENANT_CONFIG,
    KEYCLOAK_CONFIG,
    ROLES_PERMISSIONS_CONFIG,
    PUBLICATIONS,
    TALLY,
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
            EDocuments::BULLETIN_BOARDS => "export_bulletin_boards",
            EDocuments::PROTOCOL_MANAGER_KEYS => "export_protocol_manager_keys",
            EDocuments::TRUSTEES_CONFIGURATION => "trustees_configuration",
            EDocuments::TENANT_CONFIG => "tenant_configuration",
            EDocuments::KEYCLOAK_CONFIG => "keycloak_admin",
            EDocuments::ROLES_PERMISSIONS_CONFIG => "export_permissions",
            EDocuments::PUBLICATIONS => "export_publications",
            EDocuments::TALLY => "export_tally_data",
        }
    }
}

pub enum ETallyDocuments {
    TALLY_SESSION,
    TALLY_SESSION_CONTEST,
    TALLY_SESSION_EXECUTION,
    RESULTS_EVENT,
    RESULTS_ELECTION_AREA,
    RESULTS_ELECTION,
    RESULTS_CONTEST_CANDIDATE,
    RESULTS_CONTEST,
    RESULTS_AREA_CONTEST_CANDIDATE,
    RESULTS_AREA_CONTEST,
}

impl ETallyDocuments {
    pub fn to_file_name(&self) -> &str {
        match self {
            ETallyDocuments::TALLY_SESSION => "export_tally_session",
            ETallyDocuments::TALLY_SESSION_CONTEST => "export_tally_session_contest",
            ETallyDocuments::TALLY_SESSION_EXECUTION => "export_tally_session_execution",
            ETallyDocuments::RESULTS_EVENT => "export_results_event",
            ETallyDocuments::RESULTS_ELECTION_AREA => "export_results_election_area",
            ETallyDocuments::RESULTS_ELECTION => "export_results_election",
            ETallyDocuments::RESULTS_CONTEST_CANDIDATE => "export_results_contest_candidate",
            ETallyDocuments::RESULTS_CONTEST => "export_results_contest",
            ETallyDocuments::RESULTS_AREA_CONTEST_CANDIDATE => {
                "export_results_area_contest_candidate"
            }
            ETallyDocuments::RESULTS_AREA_CONTEST => "export_results_area_contest",
        }
    }
}
