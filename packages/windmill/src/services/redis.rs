// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>, Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rslock::LockManager;
use std;

pub fn get_lock_manager() -> LockManager {
    LockManager::new(vec![
        std::env::var("REDIS_URI")
            .unwrap_or_else(|_| "redis://default:yuYae4AiN3aiKoow8AhX@redis:6379/".into()),
    ])
}
