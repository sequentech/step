// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use clap::Args;
use sequent_core::services::keycloak::PartialImportSummary;
use windmill::tasks::migrate_realm_permissions::migrate_realm_permissions_now;

/// Adds the permissions of this release to every tenant realm that lacks
/// them, as Windmill's beat does when it starts. Reads the tenants from the
/// Hasura database (`HASURA_DB__*`) and signs in to Keycloak with the admin
/// client the environment names (`KEYCLOAK_URL`, `KEYCLOAK_ADMIN_CLIENT_ID`,
/// `KEYCLOAK_ADMIN_CLIENT_SECRET`). Only roles are added.
#[derive(Args)]
#[command(about = "Add this release's permissions to every tenant realm")]
pub struct MigrateRealmPermissions {}

impl MigrateRealmPermissions {
    pub fn run(&self) -> Result<()> {
        let runtime = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;
        let migrated = runtime.block_on(migrate_realm_permissions_now())?;
        print!("{}", report(&migrated));
        Ok(())
    }
}

/// One line per realm: the roles added and those it already had.
fn report(migrated: &[(String, PartialImportSummary)]) -> String {
    migrated
        .iter()
        .map(|(realm, summary)| {
            format!(
                "{realm}: {} added, {} already present\n",
                summary.added, summary.skipped
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_names_each_realm_with_its_counts() {
        let migrated = [
            (
                "tenant-a".to_string(),
                PartialImportSummary {
                    added: 21,
                    ..Default::default()
                },
            ),
            (
                "tenant-b".to_string(),
                PartialImportSummary {
                    added: 1,
                    skipped: 20,
                    ..Default::default()
                },
            ),
        ];
        assert_eq!(
            report(&migrated),
            "tenant-a: 21 added, 0 already present\ntenant-b: 1 added, 20 already present\n"
        );
        assert_eq!(report(&[]), "");
    }
}
