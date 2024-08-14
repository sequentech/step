// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::date::ISO8601;
use chrono::{DateTime, Local};
use sequent_core::types::ceremonies::Log;
use tracing::{info, instrument};

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
            "Sent transmission package xml for election '{}' ({}) and area '{}' ({}) to server '{}' ({}), signed by {}",
            election_id, election_name, area_id, area_name, server_name, server_address,
            trustees.join(", ")
        ),
    }
}
