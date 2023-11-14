// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_core::ballot::ElectionEventStatus;
use serde_json::value::Value;

pub fn get_election_event_status(status_json_opt: Option<Value>) -> Option<ElectionEventStatus> {
    status_json_opt.and_then(|status_json| serde_json::from_value(status_json).ok())
}

pub fn has_config_created(status_json_opt: Option<Value>) -> bool {
    get_election_event_status(status_json_opt)
        .map(|status| status.config_created)
        .unwrap_or(Some(false))
        .unwrap_or(false)
}
