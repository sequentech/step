use std::env;

use sequent_core::services::keycloak::{get_auth_credentials, get_tenant_realm};
use serde_json::json;
use anyhow::{anyhow, Result};

pub async fn create_resource_impl(
    resource_id: &str,
    resource_scopes: &Vec<String>,
) -> Result<String> {
    let keycloak_url =
    env::var("KEYCLOAK_URL").expect(&format!("KEYCLOAK_URL must be set"));
    let tenant_id = env::var("SUPER_ADMIN_TENANT_ID")
    .expect(&format!("SUPER_ADMIN_TENANT_ID must be set"));
    let realm = get_tenant_realm(&tenant_id);
    let client = reqwest::Client::new();
    let url = format!(
        "{}/realms/{}/authz/protection/resource_set",
        keycloak_url, realm
    );

    let body = json!({
        "name": resource_id,
        "resource_scopes": resource_scopes,
    });
    let credentials = get_auth_credentials().await?;

    let response = client
    .post(&url)
    .bearer_auth(credentials.access_token)
    .json(&body)
    .send()
    .await?;

if response.status().is_success() {
    let resource_id: String = response.json().await?;
    Ok(resource_id)
} else {
    Err(anyhow!("Failed to register resource: {}", response.text().await?))
}
}
