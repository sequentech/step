// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::routes::scheduled_event;

pub async fn process_scheduled_event(
    event: scheduled_event::ScheduledEvent,
) -> String {
    "".to_string()
}
