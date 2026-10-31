// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! The transmission and CCS log lines, dated in the event's primary zone.
use chrono::DateTime;
use chrono_tz::Tz;
use sequent_core::types::ceremonies::Log;
use tracing::{info, instrument};

#[instrument(skip_all)]
pub fn create_transmission_package_log(
    datetime: &DateTime<Tz>,
    election_id: &str,
    election_name: &str,
    area_id: &str,
    area_name: &str,
) -> Log {
    Log {
        created_date: datetime.to_rfc3339(),
        log_text: format!(
            "Created transmission package xml for election '{}' ({}) and area '{}' ({})",
            election_id, election_name, area_id, area_name
        ),
    }
}

#[instrument(skip_all)]
pub fn send_logs_to_ccs_log(
    datetime: &DateTime<Tz>,
    election_id: &str,
    election_name: &str,
    area_id: &str,
    area_name: &str,
    server_name: &str,
    server_address: &str,
) -> Log {
    Log {
        created_date: datetime.to_rfc3339(),
        log_text: format!(
            "Sent logs for election '{}' ({}) and area '{}' ({}) to server '{}' ({}).",
            election_id, election_name, area_id, area_name, server_name, server_address,
        ),
    }
}

#[instrument(skip_all)]
pub fn send_transmission_package_to_ccs_log(
    datetime: &DateTime<Tz>,
    election_id: &str,
    election_name: &str,
    area_id: &str,
    area_name: &str,
    server_name: &str,
    server_address: &str,
    trustees: Vec<String>,
) -> Log {
    Log {
        created_date: datetime.to_rfc3339(),
        log_text: format!(
            "Sent transmission package xml for election '{}' ({}) and area '{}' ({}) to server '{}' ({}), signed by [{}].",
            election_id, election_name, area_id, area_name, server_name, server_address,
            trustees.join(", ")
        ),
    }
}

#[instrument(skip_all)]
pub fn error_sending_logs_to_ccs_log(
    datetime: &DateTime<Tz>,
    election_id: &str,
    election_name: &str,
    area_id: &str,
    area_name: &str,
    server_name: &str,
    server_address: &str,
    error: &str,
) -> Log {
    Log {
        created_date: datetime.to_rfc3339(),
        log_text: format!(
            "Error sending logs for election '{}' ({}) and area '{}' ({}) to server '{}' ({}): Error '{}'",
            election_id, election_name, area_id, area_name, server_name, server_address,
            error
        ),
    }
}

#[instrument(skip_all)]
pub fn error_sending_transmission_package_to_ccs_log(
    datetime: &DateTime<Tz>,
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
        created_date: datetime.to_rfc3339(),
        log_text: format!(
            "Error sending transmission package xml for election '{}' ({}) and area '{}' ({}) to server '{}' ({}), signed by {}: Error '{}'",
            election_id, election_name, area_id, area_name, server_name, server_address,
            trustees.join(", "), error
        ),
    }
}

#[instrument(skip_all)]
pub fn sign_transmission_package_log(
    datetime: &DateTime<Tz>,
    election_id: &str,
    election_name: &str,
    area_id: &str,
    area_name: &str,
    sbei_id: &str,
) -> Log {
    Log {
        created_date: datetime.to_rfc3339(),
        log_text: format!(
            "Signed transmission package xml for election '{}' ({}) and area '{}' ({}) by sbei  '{}'",
            election_id, election_name, area_id, area_name, sbei_id
        ),
    }
}
