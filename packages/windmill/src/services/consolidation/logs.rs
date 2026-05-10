// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Human-readable [`sequent_core::types::ceremonies::Log`] lines for transmission lifecycle events.

use chrono::{DateTime, Local};
use sequent_core::services::date::ISO8601;
use sequent_core::types::ceremonies::Log;
use tracing::{info, instrument};

/// Log line recorded when a transmission package XML is generated for an election/area pair.
#[instrument(skip_all)]
pub fn create_transmission_package_log(
    datetime: &DateTime<Local>,
    election_id: &str,
    election_name: &str,
    area_id: &str,
    area_name: &str,
) -> Log {
    Log {
        created_date: ISO8601::to_string(datetime),
        log_text: format!(
            "Created transmission package xml for election '{}' ({}) and area '{}' ({})",
            election_id, election_name, area_id, area_name
        ),
    }
}

/// Log line when audit logs are POSTed to a CCS server.
#[instrument(skip_all)]
pub fn send_logs_to_ccs_log(
    datetime: &DateTime<Local>,
    election_id: &str,
    election_name: &str,
    area_id: &str,
    area_name: &str,
    server_name: &str,
    server_address: &str,
) -> Log {
    Log {
        created_date: ISO8601::to_string(datetime),
        log_text: format!(
            "Sent logs for election '{}' ({}) and area '{}' ({}) to server '{}' ({}).",
            election_id, election_name, area_id, area_name, server_name, server_address,
        ),
    }
}

/// Log line when the transmission package zip is sent to CCS, listing signing trustees.
#[instrument(skip_all)]
pub fn send_transmission_package_to_ccs_log(
    datetime: &DateTime<Local>,
    election_id: &str,
    election_name: &str,
    area_id: &str,
    area_name: &str,
    server_name: &str,
    server_address: &str,
    trustees: Vec<String>,
) -> Log {
    Log {
        created_date: ISO8601::to_string(datetime),
        log_text: format!(
            "Sent transmission package xml for election '{}' ({}) and area '{}' ({}) to server '{}' ({}), signed by [{}].",
            election_id, election_name, area_id, area_name, server_name, server_address,
            trustees.join(", ")
        ),
    }
}

/// Log line when sending audit logs to CCS fails.
#[instrument(skip_all)]
pub fn error_sending_logs_to_ccs_log(
    datetime: &DateTime<Local>,
    election_id: &str,
    election_name: &str,
    area_id: &str,
    area_name: &str,
    server_name: &str,
    server_address: &str,
    error: &str,
) -> Log {
    Log {
        created_date: ISO8601::to_string(datetime),
        log_text: format!(
            "Error sending logs for election '{}' ({}) and area '{}' ({}) to server '{}' ({}): Error '{}'",
            election_id, election_name, area_id, area_name, server_name, server_address,
            error
        ),
    }
}

/// Log line when sending the transmission package to CCS fails.
#[instrument(skip_all)]
pub fn error_sending_transmission_package_to_ccs_log(
    datetime: &DateTime<Local>,
    election_id: &str,
    election_name: &str,
    area_id: &str,
    area_name: &str,
    server_name: &str,
    server_address: &str,
    trustees: Vec<String>,
    error: &str,
) -> Log {
    Log {
        created_date: ISO8601::to_string(datetime),
        log_text: format!(
            "Error sending transmission package xml for election '{}' ({}) and area '{}' ({}) to server '{}' ({}), signed by {}: Error '{}'",
            election_id, election_name, area_id, area_name, server_name, server_address,
            trustees.join(", "), error
        ),
    }
}

/// Log line when an SBEI finishes signing the transmission package XML.
#[instrument(skip_all)]
pub fn sign_transmission_package_log(
    datetime: &DateTime<Local>,
    election_id: &str,
    election_name: &str,
    area_id: &str,
    area_name: &str,
    sbei_id: &str,
) -> Log {
    Log {
        created_date: ISO8601::to_string(datetime),
        log_text: format!(
            "Signed transmission package xml for election '{}' ({}) and area '{}' ({}) by sbei  '{}'",
            election_id, election_name, area_id, area_name, sbei_id
        ),
    }
}
