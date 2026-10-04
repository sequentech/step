// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use clap::Args;
use windmill::tasks::migrate_registration_flows::{migrate_registration_flows_now, FlowOutcome};

/// Adds the per-Post enrollment check (`enrollment-window-check`) to the
/// registration form of every event realm that lacks it, as Windmill's beat
/// does when it starts. Reads the events from the Hasura database
/// (`HASURA_DB__*`) and signs in to Keycloak with the admin client the
/// environment names (`KEYCLOAK_URL`, `KEYCLOAK_ADMIN_CLIENT_ID`,
/// `KEYCLOAK_ADMIN_CLIENT_SECRET`).
#[derive(Args)]
#[command(about = "Add the per-Post enrollment check to every event realm's registration form")]
pub struct MigrateRegistrationFlows {}

impl MigrateRegistrationFlows {
    pub fn run(&self) -> Result<()> {
        let runtime = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;
        let migrated = runtime.block_on(migrate_registration_flows_now())?;
        print!("{}", report(&migrated));
        Ok(())
    }
}

/// One line per realm with what happened.
fn report(migrated: &[(String, FlowOutcome)]) -> String {
    migrated
        .iter()
        .map(|(realm, outcome)| match outcome {
            FlowOutcome::Added { form_flow } => format!("{realm}: added to {form_flow}\n"),
            FlowOutcome::AlreadyPresent => format!("{realm}: already present\n"),
            FlowOutcome::NoRegistrationForm => format!("{realm}: no registration form\n"),
            FlowOutcome::NoRealm => format!("{realm}: no realm\n"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_names_each_realm_with_its_outcome() {
        let migrated = [
            (
                "tenant-a-event-b".to_string(),
                FlowOutcome::Added {
                    form_flow: "registration form".to_string(),
                },
            ),
            ("tenant-a-event-c".to_string(), FlowOutcome::AlreadyPresent),
            ("tenant-a-event-d".to_string(), FlowOutcome::NoRealm),
            (
                "tenant-a-event-e".to_string(),
                FlowOutcome::NoRegistrationForm,
            ),
        ];
        assert_eq!(
            report(&migrated),
            "tenant-a-event-b: added to registration form\n\
             tenant-a-event-c: already present\n\
             tenant-a-event-d: no realm\n\
             tenant-a-event-e: no registration form\n"
        );
    }
}
