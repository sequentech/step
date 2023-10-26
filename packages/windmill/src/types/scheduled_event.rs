// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde_json::Value;
use serde::{Deserialize, Serialize};
use strum_macros::Display;
use strum_macros::EnumString;

#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum EventProcessors {
    CREATE_REPORT,
    UPDATE_VOTING_STATUS,
    CREATE_BOARD,
    CREATE_KEYS,
    SET_PUBLIC_KEY,
    CREATE_BALLOT_STYLE,
    INSERT_BALLOTS,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScheduledEvent {
    pub id: String,
    pub tenant_id: Option<String>,
    pub election_event_id: Option<String>,
    pub created_at: Option<String>,
    pub stopped_at: Option<String>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub event_processor: Option<EventProcessors>,
    pub cron_config: Option<String>,
    pub event_payload: Option<Value>,
    pub created_by: Option<String>,
}
