use anyhow::{anyhow, Result};
use sequent_core::services::keycloak::KeycloakAdminClient;
use tracing::instrument;

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
