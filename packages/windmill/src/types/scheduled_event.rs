// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::Display;
use strum_macros::EnumString;

#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum EventProcessors {
    CREATE_REPORT,
    SEND_TEMPLATE,
    START_ELECTION,
    END_ELECTION,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct CronConfig {
    pub cron: Option<String>,
    pub scheduled_date: Option<String>,
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
    pub cron_config: Option<CronConfig>,
    pub event_payload: Option<Value>,
}
