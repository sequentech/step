// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Board name utilities for B4.
//!
//! This module provides utilities for working with board names, including
//! verification against JWT claims for tenant and event ID matching.

use anyhow::{anyhow, Result};
use tracing::instrument;

/// Extracted tenant and event IDs from a board name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardNameParts {
    /// The slug prefix (e.g., "dev", "prod").
    pub slug: String,
    /// The tenant ID portion (first 17 chars of tenant UUID, no dashes).
    pub tenant_prefix: String,
    /// The election event ID (no dashes).
    pub event_id: String,
}

/// Extracts tenant and event ID parts from a board name.
///
/// Board name format: `{slug}tenant{tenant_chars}event{election_event_id_no_dashes}`
/// where `tenant_chars` is the first 17 characters of tenant_id with dashes removed.
///
/// # Arguments
/// * `board_name` - The board name to parse
///
/// # Returns
/// * `Ok(BoardNameParts)` - The extracted parts
/// * `Err` - If the board name doesn't match the expected format
///
/// # Example
/// ```ignore
/// let parts = extract_board_name_parts("devtenant90505c8a23a94cdfaevent388b3effe5834a5682b70ad15eaa409a")?;
/// assert_eq!(parts.slug, "dev");
/// assert_eq!(parts.tenant_prefix, "90505c8a23a94cdfa");
/// assert_eq!(parts.event_id, "388b3effe5834a5682b70ad15eaa409a");
/// ```
#[instrument(level = "trace", err)]
pub fn extract_board_name_parts(board_name: &str) -> Result<BoardNameParts> {
    // Find "tenant" marker
    let tenant_pos = board_name
        .find("tenant")
        .ok_or_else(|| anyhow!("Board name missing 'tenant' marker: {board_name}"))?;

    // Find "event" marker (must come after "tenant")
    let event_marker = "event";
    let event_pos = board_name[tenant_pos..]
        .find(event_marker)
        .map(|pos| tenant_pos + pos)
        .ok_or_else(|| anyhow!("Board name missing 'event' marker: {board_name}"))?;

    // Extract parts
    let slug = &board_name[..tenant_pos];
    let tenant_start = tenant_pos + "tenant".len();
    let tenant_prefix = &board_name[tenant_start..event_pos];
    let event_id = &board_name[event_pos + event_marker.len()..];

    // Validate tenant prefix length (should be 17 chars)
    if tenant_prefix.len() != 17 {
        return Err(anyhow!(
            "Invalid tenant prefix length: expected 17, got {} in board name: {board_name}",
            tenant_prefix.len()
        ));
    }

    // Validate event ID is not empty
    if event_id.is_empty() {
        return Err(anyhow!("Empty event ID in board name: {board_name}"));
    }

    Ok(BoardNameParts {
        slug: slug.to_string(),
        tenant_prefix: tenant_prefix.to_string(),
        event_id: event_id.to_string(),
    })
}

/// Converts a tenant UUID to its board name prefix format.
///
/// Removes dashes and takes the first 17 characters.
#[instrument(level = "trace")]
pub fn tenant_id_to_prefix(tenant_id: &str) -> String {
    tenant_id.chars().filter(|&c| c != '-').take(17).collect()
}

/// Verifies that a board name matches the given tenant ID.
///
/// # Arguments
/// * `board_name` - The board name to verify
/// * `tenant_id` - The tenant UUID from JWT claims (with dashes)
///
/// # Returns
/// * `Ok(())` - If the board name's tenant prefix matches
/// * `Err` - If there's a mismatch or the board name is invalid
#[instrument(level = "trace", err)]
pub fn verify_board_tenant(board_name: &str, tenant_id: &str) -> Result<()> {
    let parts = extract_board_name_parts(board_name)?;
    let expected_prefix = tenant_id_to_prefix(tenant_id);

    if parts.tenant_prefix != expected_prefix {
        return Err(anyhow!(
            "Tenant mismatch: board tenant prefix '{}' does not match expected '{expected_prefix}' from tenant ID '{tenant_id}'",
            parts.tenant_prefix
        ));
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    const TEST_TENANT_ID: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
    const TEST_EVENT_ID: &str = "388b3eff-e583-4a56-82b7-0ad15eaa409a";

    #[test]
    fn test_extract_board_name_parts_valid() {
        let board_name = "devtenant90505c8a23a94cdfaevent388b3effe5834a5682b70ad15eaa409a";
        let parts = extract_board_name_parts(board_name).unwrap();

        assert_eq!(parts.slug, "dev");
        assert_eq!(parts.tenant_prefix, "90505c8a23a94cdfa");
        assert_eq!(parts.event_id, "388b3effe5834a5682b70ad15eaa409a");
    }

    #[test]
    fn test_extract_board_name_parts_different_slug() {
        let board_name = "prodtenant90505c8a23a94cdfaevent388b3effe5834a5682b70ad15eaa409a";
        let parts = extract_board_name_parts(board_name).unwrap();

        assert_eq!(parts.slug, "prod");
        assert_eq!(parts.tenant_prefix, "90505c8a23a94cdfa");
    }

    #[test]
    fn test_extract_board_name_parts_missing_tenant() {
        let board_name = "devevent388b3effe5834a5682b70ad15eaa409a";
        let result = extract_board_name_parts(board_name);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing 'tenant'"));
    }

    #[test]
    fn test_extract_board_name_parts_missing_event() {
        let board_name = "devtenant90505c8a23a94cdfa";
        let result = extract_board_name_parts(board_name);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing 'event'"));
    }

    #[test]
    fn test_extract_board_name_parts_invalid_tenant_length() {
        // Tenant prefix too short (only 10 chars instead of 17)
        let board_name = "devtenant1234567890event388b3effe5834a5682b70ad15eaa409a";
        let result = extract_board_name_parts(board_name);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid tenant prefix length"));
    }

    #[test]
    fn test_tenant_id_to_prefix() {
        let prefix = tenant_id_to_prefix(TEST_TENANT_ID);
        assert_eq!(prefix, "90505c8a23a94cdfa");
        assert_eq!(prefix.len(), 17);
    }

    #[test]
    fn test_verify_board_tenant_valid() {
        let board_name = "devtenant90505c8a23a94cdfaevent388b3effe5834a5682b70ad15eaa409a";
        let result = verify_board_tenant(board_name, TEST_TENANT_ID);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_board_tenant_mismatch() {
        let board_name = "devtenant90505c8a23a94cdfaevent388b3effe5834a5682b70ad15eaa409a";
        let wrong_tenant = "12345678-1234-1234-1234-123456789012";
        let result = verify_board_tenant(board_name, wrong_tenant);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Tenant mismatch"));
    }

}

