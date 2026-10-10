// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use keycloak::KeycloakError;
use uuid::Uuid;

/// The Keycloak client interpolates identifiers into URL paths without
/// escaping reserved characters. Reject path, query, fragment and encoding
/// controls before building a request so an identifier stays in its segment.
pub fn validate_keycloak_path_segment(
    segment: &str,
) -> Result<(), KeycloakError> {
    if segment.is_empty()
        || matches!(segment, "." | "..")
        || segment.starts_with(' ')
        || segment.ends_with(' ')
        || segment.chars().any(|character| {
            matches!(character, '/' | '\\' | '?' | '#' | '%')
                || character.is_control()
        })
    {
        return Err(KeycloakError::HttpFailure {
            status: 400,
            body: None,
            text: "Invalid Keycloak path segment".to_string(),
        });
    }
    Ok(())
}

/// Validate tenant/event identifiers at the API boundary before realm creation.
pub fn validate_keycloak_scope(
    tenant_id: &str,
    event_id: Option<&str>,
) -> Result<()> {
    Uuid::parse_str(tenant_id).context("tenant_id must be a UUID")?;
    if let Some(event_id) = event_id {
        Uuid::parse_str(event_id)
            .context("election_event_id must be a UUID")?;
    }
    Ok(())
}
