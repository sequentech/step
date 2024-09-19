use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak::KeycloakAdminClient;
use tracing::instrument;

use crate::postgres::election_event::delete_election_event;

use super::database::get_hasura_pool;

#[instrument(err)]
pub async fn delete_keycloak_realm(realm: &str) -> Result<()> {
    let client = KeycloakAdminClient::new().await?;
    let _ = client
        .client
        .realm_delete(&realm)
        .await
        .map_err(|err| anyhow!("Keycloak error: {:?}", err));
    //TODO: delete linked S3 artifacts
    Ok(())
}

#[instrument(err)]
pub async fn delete_election_event_db(tenant_id: String, election_event_id: String) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    delete_election_event(&hasura_transaction, &tenant_id, &election_event_id).await?;
    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;
    Ok(())
}
