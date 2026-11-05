// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! The Vitrine sidecar's grants on election event databases against a real server:
//! `ELECTORAL_LOG_PG_*` name a role that may create roles and databases. A binary of its
//! own, because it points `ELECTORAL_LOG_PG_VITRINE_USER` at a role it creates.

use anyhow::Result;
use electoral_log::adapters::{
    events::EventDatabases,
    postgres::{PostgresConnection, VITRINE_USER_ENV},
};
use std::env;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_PG_* with a role that may create roles and databases"]
async fn the_vitrine_role_may_keep_its_schema_in_event_databases() -> Result<()> {
    let admin = PostgresConnection::from_env()?;
    let client = admin.client_of(admin.database()).await?;
    let vitrine = format!("elvitrine_{}", &Uuid::new_v4().simple().to_string()[..8]);
    client
        .batch_execute(&format!("CREATE ROLE {vitrine} NOLOGIN"))
        .await?;
    env::set_var(VITRINE_USER_ENV, &vitrine);

    let databases = EventDatabases::from_env()?;
    let (tenant, event) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
    databases.create_event(&tenant, &event).await?;
    let database = databases.database_name(&event)?;
    let may = |privilege: &'static str| {
        let (vitrine, database, client) = (&vitrine, &database, &client);
        async move {
            let row = client
                .query_one(
                    "SELECT has_database_privilege($1, $2, $3)",
                    &[vitrine, database, &privilege],
                )
                .await?;
            anyhow::Ok(row.get::<_, bool>(0))
        }
    };
    assert!(may("CONNECT").await?);
    assert!(may("CREATE").await?);
    // PUBLIC still may not connect.
    let public = client
        .query_one(
            "SELECT coalesce(bool_or(acl.grantee = 0 AND acl.privilege_type = 'CONNECT'), false) \
             FROM pg_database, aclexplode(datacl) acl WHERE datname = $1",
            &[&database],
        )
        .await?;
    assert!(!public.get::<_, bool>(0));

    databases.drop_event(&tenant, &event).await?;
    databases.catalog().close();
    drop(databases);
    client
        .batch_execute(&format!("DROP ROLE {vitrine}"))
        .await?;
    Ok(())
}
