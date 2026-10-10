// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use clap::Args;
use sequent_core::services::keycloak::PartialImportSummary;
use windmill::tasks::migrate_realm_permissions::{
    migrate_realm_permissions_now, GroupGrant, RealmMigration,
};

/// Adds the permissions of this release to every tenant realm that lacks
/// them, as Windmill's beat does when it starts. Reads the tenants from the
/// Hasura database (`HASURA_DB__*`) and signs in to Keycloak with the admin
/// client the environment names (`KEYCLOAK_URL`, `KEYCLOAK_ADMIN_CLIENT_ID`,
/// `KEYCLOAK_ADMIN_CLIENT_SECRET`). Groups get only the messaging roles the
/// realm template gives them.
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

/// One line per realm: the roles added and those it already had, then one
/// line per group that was granted roles.
fn report(migrated: &[RealmMigration]) -> String {
    migrated
        .iter()
        .map(|migration| {
            let mut lines = format!(
                "{}: {} added, {} already present\n",
                migration.realm, migration.roles.added, migration.roles.skipped
            );
            for GroupGrant { group, roles } in &migration.granted {
                lines.push_str(&format!("  {group}: granted {}\n", roles.join(", ")));
            }
            lines
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_names_each_realm_with_its_counts() {
        let migrated = [
            RealmMigration {
                realm: "tenant-a".to_string(),
                roles: PartialImportSummary {
                    added: 21,
                    ..Default::default()
                },
                granted: vec![GroupGrant {
                    group: "/admin".to_string(),
                    roles: vec![
                        "messaging-config-read".to_string(),
                        "messaging-config-write".to_string(),
                    ],
                }],
            },
            RealmMigration {
                realm: "tenant-b".to_string(),
                roles: PartialImportSummary {
                    added: 1,
                    skipped: 20,
                    ..Default::default()
                },
                granted: vec![],
            },
        ];
        assert_eq!(
            report(&migrated),
            "tenant-a: 21 added, 0 already present\n  /admin: granted messaging-config-read, messaging-config-write\ntenant-b: 1 added, 20 already present\n"
        );
        assert_eq!(report(&[]), "");
    }
}
