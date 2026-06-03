// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
#[cfg(feature = "log")]
use tracing::info;

pub const DEV_APP_VERSION: &str = "dev";
pub const ENV_VAR_APP_VERSION: &str = "APP_VERSION";
pub const ENV_VAR_APP_HASH: &str = "APP_HASH";
pub const VERSION_KEY: &str = "version";
pub const HISTORICAL_DEFAULT_VERSION: &str = "9.0.0";

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

    let (current_major, current_minor, _) = extract_semver(current_version)
        .ok_or_else(|| anyhow!("Could not parse current version"))?;
    let (imported_major, imported_minor, _) = extract_semver(imported_version)
        .ok_or_else(|| anyhow!("Could not parse imported version"))?;

    if (current_major != imported_major) || (current_minor < imported_minor) {
        return Err(anyhow!(
            "Version mismatch: Imported version {} is not compatible with current version {}. Please upgrade your system.",
            imported_version,
            current_version
        ));
    }
    Ok(())
}

/// Extract the semantic version from the given string.
///
/// Major version is required, and None will be returned if missing.
/// Missing minor or patch versions will default to 0; pre-release tags are ignored (-*).
fn extract_semver(input: &str) -> Option<(u64, u64, u64)> {
    // Trim optional 'v' or 'V' prefix
    let trimmed = input.trim_start_matches(['v', 'V']);

    // Split off any pre-release suffix (e.g. "-alpha", "-rc1") by splitting on '-'
    let version_part = trimmed.split('-').next().unwrap_or("");

    // Split on '.' and parse up to three parts; missing components default to 0
    let mut parts = version_part.split('.').map(|s| s.parse().ok());
    let major: u64 = parts.next()??;
    let minor: u64 = parts.next().flatten().unwrap_or(0);
    let patch: u64 = parts.next().flatten().unwrap_or(0);

    Some((major, minor, patch))
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
        // Imported: 1.0.0, Current: 1.1.0
        assert!(check_version_compatibility("1.0.0", "1.1.0").is_ok());

        // Imported: 10, Current: 11
        assert!(check_version_compatibility("10.5.5", "10.5.7").is_ok());
    }

    #[test]
    fn test_forward_compatibility_rejection() {
        // Importing a NEWER version into an OLDER system should FAIL
        // Imported: 1.1.0, Current: 1.0.0
        let result = check_version_compatibility("1.1.0", "1.0.0");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not compatible"));

        // Patch version is accepted.
        let result = check_version_compatibility("1.1.1", "1.1.0");
        assert!(result.is_ok());
    }

    #[test]
    fn test_major_mismatch_rejection() {
        // Importing anything with a different major version should fail,
        //  be it older or newer.
        let result = check_version_compatibility("1.0.0", "2.0.0");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not compatible"));

        let result = check_version_compatibility("11.0.0", "10.0.0");
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
    fn test_extract_semver_logic() {
        // Standard semver
        assert_eq!(extract_semver("1.2.3"), Some((1, 2, 3)));
        assert_eq!(extract_semver("10.0.0"), Some((10, 0, 0)));
        assert_eq!(extract_semver("0.5.9"), Some((0, 5, 9)));

        // With prefixes
        assert_eq!(extract_semver("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(extract_semver("V2.0.0"), Some((2, 0, 0)));

        // With suffixes (alpha, beta, rc)
        assert_eq!(extract_semver("1.0.0-alpha"), Some((1, 0, 0)));
        assert_eq!(extract_semver("3.0.0-rc1"), Some((3, 0, 0)));
        assert_eq!(extract_semver("v4-beta"), Some((4, 0, 0)));

        // Edge cases
        assert_eq!(extract_semver("2"), Some((2, 0, 0))); // Just a number
        assert_eq!(extract_semver("2.none.1"), Some((2, 0, 1))); // Parse error in the midle
        assert_eq!(extract_semver("q.1.0"), None); // Major version is problematic
        assert_eq!(extract_semver("not_a_number"), None);
        assert_eq!(extract_semver(""), None);
        assert_eq!(extract_semver("v"), None);

        // Ensure it stops at non-digits
        assert_eq!(extract_semver("5startswithnumber"), None);
    }
}
