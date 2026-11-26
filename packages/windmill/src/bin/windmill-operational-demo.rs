// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Never run as an ordinary worker or load its ambient dotenv file.
    if std::env::var("OPERATIONAL_SIGNING_DEMO_WORKER").as_deref() != Ok("1") {
        return Err("The isolated signing worker requires its dedicated launcher".into());
    }
    windmill::app::main()
}
