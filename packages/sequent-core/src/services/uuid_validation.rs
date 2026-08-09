// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::anyhow;
use uuid::Uuid;

/// Parses and validates that the given string is a valid v4 UUID.<br>
/// Returns the parsed `Uuid` on success, or an error if the string is not
/// a valid UUID or is not version 4 (random).
pub fn parse_uuid_v4(value: &str) -> anyhow::Result<Uuid> {
    let uuid = Uuid::parse_str(value)
        .map_err(|e| anyhow!("invalid UUID '{}': {}", value, e))?;

    match uuid.get_version() {
        Some(uuid::Version::Random) => Ok(uuid),
        other => {
            Err(anyhow!("UUID '{}' is not v4 (version: {:?})", value, other))
        }
    }
}

/// Parses and validates that the given string is a valid v4 UUID,
/// returning a descriptive error that includes the field name.
pub fn parse_uuid_v4_field(
    value: &str,
    field_name: &str,
) -> anyhow::Result<Uuid> {
    parse_uuid_v4(value).map_err(|e| {
        anyhow!("invalid v4 UUID for field '{}': {}", field_name, e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_v4_uuid() {
        let uuid = Uuid::new_v4().to_string();
        assert!(parse_uuid_v4(&uuid).is_ok());
    }

    #[test]
    fn test_valid_v4_uuid_uppercase() {
        let uuid = Uuid::new_v4().to_string().to_uppercase();
        assert!(parse_uuid_v4(&uuid).is_ok());
    }

    #[test]
    fn test_invalid_uuid_string() {
        assert!(parse_uuid_v4("not-a-uuid").is_err());
    }

    #[test]
    fn test_empty_string() {
        assert!(parse_uuid_v4("").is_err());
    }

    #[test]
    fn test_nil_uuid_is_not_v4() {
        assert!(parse_uuid_v4("00000000-0000-0000-0000-000000000000").is_err());
    }

    #[test]
    fn test_parse_uuid_v4_field_error_includes_field_name() {
        let err = parse_uuid_v4_field("bad", "tenant_id").unwrap_err();
        assert!(err.to_string().contains("tenant_id"));
    }
}
