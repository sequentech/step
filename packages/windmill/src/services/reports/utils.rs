// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use sequent_core::services::s3::get_minio_url;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::number_format::NumberFormatPolicy;
use std::env;
use tracing::{instrument, warn};

/// Function to get the public assets path environment variable
#[instrument(err, skip_all)]
pub fn get_public_assets_path_env_var() -> Result<String> {
    env::var("PUBLIC_ASSETS_PATH").map_err(|_| anyhow!("PUBLIC_ASSETS_PATH env var missing"))
}

/// Helper function to get public asset templates
#[instrument(err, skip_all)]
pub async fn get_public_asset_template(filename: &str) -> Result<String> {
    let public_asset_path = get_public_assets_path_env_var()?;

    let minio_endpoint_base = get_minio_url().with_context(|| "Error getting minio endpoint")?;

    let template_url = format!("{}/{}/{}", minio_endpoint_base, public_asset_path, filename);

    let client = reqwest::Client::new();
    let response = client
        .get(&template_url)
        .send()
        .await
        .with_context(|| format!("Error sending request for template {}", filename))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(anyhow!("File not found: {}", template_url));
    }
    if !response.status().is_success() {
        return Err(anyhow!(
            "Unexpected response status: {:?}",
            response.status()
        ));
    }

    let template_hbs: String = response
        .text()
        .await
        .with_context(|| format!("Error reading the template response for {}", filename))?;

    Ok(template_hbs)
}

/// The number format the reports of `election_event` write their figures
/// in, or `None` for the default. An event whose presentation can't be read
/// gets the default too, so that its reports still render.
pub fn get_number_format_policy(election_event: &ElectionEvent) -> Option<NumberFormatPolicy> {
    match election_event.get_presentation() {
        Ok(presentation) => presentation.and_then(|presentation| presentation.number_format_policy),
        Err(err) => {
            warn!(
                "Can't read the presentation of election event {}, using the default number format: {err}",
                election_event.id
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn election_event(presentation: Option<Value>) -> ElectionEvent {
        ElectionEvent {
            id: "event".to_string(),
            created_at: None,
            updated_at: None,
            labels: None,
            annotations: None,
            tenant_id: "tenant".to_string(),
            description: None,
            presentation,
            bulletin_board_reference: None,
            is_archived: false,
            voting_channels: None,
            status: None,
            user_boards: None,
            encryption_protocol: "RSA256".to_string(),
            is_audit: None,
            audit_election_event_id: None,
            public_key: None,
            statistics: None,
            external_id: None,
        }
    }

    #[test]
    fn reports_follow_the_number_format_policy_of_their_election_event() {
        let event = election_event(Some(json!({ "number_format_policy": "period-comma" })));

        assert_eq!(
            get_number_format_policy(&event),
            Some(NumberFormatPolicy::PeriodComma)
        );
    }

    #[test]
    fn events_without_a_number_format_policy_use_the_default() {
        for presentation in [
            None,
            Some(json!({})),
            Some(json!({ "number_format_policy": null })),
        ] {
            assert_eq!(
                get_number_format_policy(&election_event(presentation)),
                None
            );
        }
    }

    #[test]
    fn an_unreadable_number_format_policy_uses_the_default() {
        let event = election_event(Some(json!({ "number_format_policy": "unknown" })));

        assert_eq!(get_number_format_policy(&event), None);
    }
}
