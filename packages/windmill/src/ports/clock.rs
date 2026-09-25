// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use chrono::{DateTime, Local};
use uuid::Uuid;

/// The current time.
pub trait Clock: Sync {
    fn now(&self) -> DateTime<Local>;
}

/// New unique identifiers.
pub trait IdGenerator: Sync {
    fn new_id(&self) -> Uuid;
}
