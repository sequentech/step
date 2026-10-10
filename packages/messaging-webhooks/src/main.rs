// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Starts the webhook component; see the library for what it does.

use messaging_webhooks::{build, Config};

#[rocket::launch]
fn rocket() -> _ {
    messaging_webhooks::logging::init();
    match Config::from_env().and_then(build) {
        Ok(rocket) => rocket,
        Err(error) => {
            tracing::error!("messaging-webhooks can't start: {error:#}");
            std::process::exit(1);
        }
    }
}
