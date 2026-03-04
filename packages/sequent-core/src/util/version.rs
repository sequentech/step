// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
#[cfg(feature = "log")]
use tracing::info;

pub const DEV_APP_VERSION: &str = "dev";
pub const ENV_VAR_APP_VERSION: &str = "APP_VERSION";
pub const ENV_VAR_APP_HASH: &str = "APP_HASH";

pub fn check_version_compatibility(
    imported_version: &str,
    current_version: &str,
) -> Result<()> {
    #[cfg(feature = "log")]
    info!(
        "Checking version compatibility - Current: {}, Imported: {}",
        current_version, imported_version
    );

    // If current version is DEV_APP_VERSION, allow any import
    if current_version == DEV_APP_VERSION {
        #[cfg(feature = "log")]
        info!("Current version is 'dev', allowing import");
        return Ok(());
    }

    if imported_version == DEV_APP_VERSION {
        #[cfg(feature = "log")]
        info!("Imported version is 'dev' while system is not in dev mode, rejecting import");
        return Err(anyhow!("Imported version is 'dev', which is not compatible with current version {}. Please use a different version.", current_version));
    }

    let current_major_parsed = extract_major(&current_version)
        .ok_or_else(|| anyhow!("Could not parse current version"))?;
    let imported_major_parsed = extract_major(imported_version)
        .ok_or_else(|| anyhow!("Could not parse imported version"))?;

    if current_major_parsed < imported_major_parsed {
        return Err(anyhow!(
            "Version mismatch: Imported version {} is not compatible with current version {}. Please upgrade your system.",
            imported_version,
            current_version
        ));
    }
    Ok(())
}

fn extract_major(input: &str) -> Option<u64> {
    // Trim optional 'v' or 'V' prefix
    let trimmed = input.trim_start_matches(|c| c == 'v' || c == 'V');

    // We take characters from the start as long as they are digits.
    // This stops at the first dot '.', hyphen '-', or any non-digit.
    let major_str: String =
        trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();

    // Parse the result into a u64
    // If the string was empty (e.g., input was "invalid"), this returns None.
    major_str.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================
    // Public API Tests: check_version_compatibility
    // ==========================================

    #[test]
    fn test_current_version_is_dev() {
        // If current system is DEV_APP_VERSION, it should accept anything
        assert!(check_version_compatibility("1.0.0", DEV_APP_VERSION).is_ok());
        assert!(
            check_version_compatibility("99.99.99", DEV_APP_VERSION).is_ok()
        );
        assert!(
            check_version_compatibility(DEV_APP_VERSION, DEV_APP_VERSION)
                .is_ok()
        );
    }

    #[test]
    fn test_imported_version_is_dev_rejected() {
        // If importing DEV_APP_VERSION into a non-dev system, it must fail
        let result = check_version_compatibility(DEV_APP_VERSION, "1.0.0");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Imported version is 'dev', which is not compatible with current version 1.0.0. Please use a different version."
        );
    }

    #[test]
    fn test_exact_match_versions() {
        assert!(check_version_compatibility("1.0.0", "1.0.0").is_ok());
        assert!(check_version_compatibility("2.5.1", "2.5.1").is_ok());
    }

    #[test]
    fn test_backward_compatibility() {
        // Importing an OLDER version into a NEWER system should be OK
        // Imported: 1, Current: 2
        assert!(check_version_compatibility("1.0.0", "2.0.0").is_ok());

        // Imported: 10, Current: 11
        assert!(check_version_compatibility("10.5.5", "11.0.0").is_ok());
    }

    #[test]
    fn test_forward_compatibility_rejection() {
        // Importing a NEWER version into an OLDER system should FAIL
        // Imported: 2, Current: 1
        let result = check_version_compatibility("2.0.0", "1.0.0");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not compatible"));
    }

    #[test]
    fn test_parsing_failures() {
        // Invalid current version
        let res_current = check_version_compatibility("1.0.0", "invalid_ver");
        assert!(res_current.is_err());
        assert!(res_current
            .unwrap_err()
            .to_string()
            .contains("Could not parse current version"));

        // Invalid imported version
        let res_imported = check_version_compatibility("invalid_ver", "1.0.0");
        assert!(res_imported.is_err());
        assert!(res_imported
            .unwrap_err()
            .to_string()
            .contains("Could not parse imported version"));
    }

    #[test]
    fn test_version_prefixes() {
        // Handling 'v' or 'V' prefixes
        // Imported v1 (1) into Current 1 -> OK
        assert!(check_version_compatibility("v1.0.0", "1.0.0").is_ok());

        // Imported V2 (2) into Current v1 (1) -> Error
        assert!(check_version_compatibility("V2.0.0", "v1.0.0").is_err());
    }

    // ==========================================
    // Internal Helper Tests: extract_major
    // ==========================================

    #[test]
    fn test_extract_major_logic() {
        // Standard semver
        assert_eq!(extract_major("1.2.3"), Some(1));
        assert_eq!(extract_major("10.0.0"), Some(10));
        assert_eq!(extract_major("0.5.9"), Some(0));

        // With prefixes
        assert_eq!(extract_major("v1.2.3"), Some(1));
        assert_eq!(extract_major("V2.0.0"), Some(2));

        // With suffixes (alpha, beta, rc)
        assert_eq!(extract_major("1.0.0-alpha"), Some(1));
        assert_eq!(extract_major("3.0.0-rc1"), Some(3));
        assert_eq!(extract_major("v4-beta"), Some(4));

        // Edge cases
        assert_eq!(extract_major("2"), Some(2)); // Just a number
        assert_eq!(extract_major("not_a_number"), None);
        assert_eq!(extract_major(""), None);
        assert_eq!(extract_major("v"), None);

        // Ensure it stops at non-digits
        assert_eq!(extract_major("5startswithnumber"), Some(5));
    }
}
