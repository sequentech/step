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
    BULLETIN_BOARDS,
    PROTOCOL_MANAGER_KEYS,
    TRUSTEES_CONFIGURATION,
    TENANT_CONFIG,
    KEYCLOAK_CONFIG,
    ROLES_PERMISSIONS_CONFIG,
    PUBLICATIONS,
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
        }
    }
}
