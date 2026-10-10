// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Election event databases created by a provisioning role against a real server:
//! `ELECTORAL_LOG_PG_*` name a role that may create roles and databases, which sets up
//! an application role without `CREATEDB`. A binary of its own, because it points the
//! `ELECTORAL_LOG_PG_*` variables at those roles.

use anyhow::Result;
use electoral_log::{
    adapters::{events::EventDatabases, postgres::PostgresConnection},
    ports::ElectoralLogStore,
};
use std::env;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_PG_* with a role that may create roles and databases"]
async fn a_provisioning_role_creates_the_databases_the_application_role_owns() -> Result<()> {
    let admin = PostgresConnection::from_env()?;
    let client = admin.client_of(admin.database()).await?;
    let suffix = &Uuid::new_v4().simple().to_string()[..8];
    let (app, provisioner, base) = (
        format!("elapp_{suffix}"),
        format!("elprovisioner_{suffix}"),
        format!("elbase_{suffix}"),
    );
    client
        .batch_execute(&format!(
            "CREATE ROLE {app} LOGIN NOCREATEDB PASSWORD 'app';
             CREATE ROLE {provisioner} LOGIN CREATEDB PASSWORD 'provisioner';
             GRANT {app} TO {provisioner};"
        ))
        .await?;
    client
        .batch_execute(&format!("CREATE DATABASE {base} OWNER {app}"))
        .await?;
    env::set_var("ELECTORAL_LOG_PG_USER", &app);
    env::set_var("ELECTORAL_LOG_PG_PASSWORD", "app");
    env::set_var("ELECTORAL_LOG_PG_DATABASE", &base);
    env::set_var("ELECTORAL_LOG_PG_PROVISIONING_USER", &provisioner);
    env::set_var("ELECTORAL_LOG_PG_PROVISIONING_PASSWORD", "provisioner");

    let databases = EventDatabases::from_env()?;
    let (tenant, event) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
    let store = databases.create_event(&tenant, &event).await?;
    let board = format!(
        "testtenant{}event{}",
        &tenant.replace('-', "")[..17],
        event.replace('-', "")
    );
    store.create_board(&board).await?;
    let database = databases.database_name(&event)?;
    let owner: String = client
        .query_one(
            "SELECT pg_get_userbyid(datdba)::text FROM pg_database WHERE datname = $1",
            &[&database],
        )
        .await?
        .get(0);
    assert_eq!(owner, app);
    // The application role owns the event's database but cannot create one.
    let app_client = databases.connection().client_of(&base).await?;
    assert!(app_client
        .batch_execute(&format!("CREATE DATABASE elrefused_{suffix}"))
        .await
        .is_err());
    drop(app_client);

    databases.drop_event(&tenant, &event).await?;
    assert!(!databases.has_event(&event).await?);
    assert!(client
        .query_opt("SELECT 1 FROM pg_database WHERE datname = $1", &[&database])
        .await?
        .is_none());

    databases.catalog().close();
    drop(store);
    drop(databases);
    client
        .batch_execute(&format!("DROP DATABASE {base} WITH (FORCE)"))
        .await?;
    client
        .batch_execute(&format!("DROP ROLE {provisioner}; DROP ROLE {app};"))
        .await?;
    Ok(())
}
