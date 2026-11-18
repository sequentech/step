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

/// The platform release line this source tree builds, as the bundles it writes
/// declare it.
///
/// A bundle's `version` is what [`check_version_compatibility`] compares
/// against the importing platform's `APP_VERSION`: same major, and a minor no
/// newer than the platform's. A bundle this crate writes has exactly the shape
/// this crate's `ImportElectionEventSchema` reads, so it must declare the line
/// that importer ships in, never an older one: a v10 label on a v11 bundle is
/// refused by every v11 platform before a single field is read. That is how
/// the OVCS Architect's downloads came to be refused on v11.0.0-rc.10.
///
/// The crate's own `CARGO_PKG_VERSION` (`0.1.0`) is not a platform version,
/// and the release tags are not visible to a build, so the line is stated
/// here. Bump it when the branch starts a new major or minor line; the tests
/// below pin it against the platform versions that must accept it.
pub const PLATFORM_VERSION: &str = "v11.0.0";

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

/// Whether [`check_version_compatibility`] refuses `imported` only because its
/// **major** version differs from `current`'s, in either direction.
///
/// A classifier, not a second rule: the strict check above is unchanged for
/// every caller. The election-event import uses this to report a different
/// major as a warning instead of refusing the file (meta#14760); anything else
/// the strict check refuses (an unreadable version, `dev` on one side only, a
/// newer minor of the same major) is not a major mismatch and still refuses.
pub fn major_mismatch(imported_version: &str, current_version: &str) -> bool {
    match (
        strict_major(imported_version),
        strict_major(current_version),
    ) {
        (Some(imported), Some(current)) => imported != current,
        _ => false,
    }
}

/// The major of a version whose numeric part is well formed: an optional
/// `v`/`V`, then one to three dot-separated runs of digits, then optionally a
/// `-` pre-release suffix (`v11.0.0-rc.10`) and `+` build metadata
/// (`v12.0.0+build.7`, `v12.0.0-rc.1+build.7`).
///
/// Strict numeric-version parsing, not a complete SemVer validator: the
/// numbers are checked, and the contents of a pre-release or build suffix are
/// not. Stricter than [`extract_semver`], which reads
/// `v12.bad.0` as 12.0.0: a version that malformed is not a "different
/// major" here, so it stays refused rather than becoming a warning. `dev`
/// has no major.
fn strict_major(version: &str) -> Option<u64> {
    let unprefixed = version.strip_prefix(['v', 'V']).unwrap_or(version);
    // Build metadata first: it follows the pre-release when both are there.
    let unbuilt = unprefixed
        .split_once('+')
        .map_or(unprefixed, |(core, _)| core);
    let numbers = unbuilt
        .split_once('-')
        .map_or(unbuilt, |(numbers, _)| numbers);
    let parts: Vec<&str> = numbers.split('.').collect();
    if parts.is_empty()
        || parts.len() > 3
        || parts.iter().any(|part| {
            part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit())
        })
    {
        return None;
    }
    parts[0].parse().ok()
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
    fn the_platform_version_is_accepted_by_its_own_release_line() {
        // The deployed OVCS release, and its final release.
        for platform in ["v11.0.0-rc.10", "v11.0.0", "11.0.0"] {
            assert!(
                check_version_compatibility(PLATFORM_VERSION, platform).is_ok(),
                "{PLATFORM_VERSION} must import on {platform}"
            );
        }
        // Prior lines must refuse it as a version mismatch, not a serde error.
        for platform in ["v10.0.0", "v10.1.0-rc.4", "v9.5.0"] {
            assert!(
                check_version_compatibility(PLATFORM_VERSION, platform)
                    .unwrap_err()
                    .to_string()
                    .contains("Version mismatch"),
                "{platform} must refuse a {PLATFORM_VERSION} bundle"
            );
        }
        // The label that shipped before: refused by the platform it ran on.
        assert!(
            check_version_compatibility("v10.0.0", "v11.0.0-rc.10").is_err()
        );
    }

    #[test]
    fn a_major_mismatch_is_a_different_major_in_either_direction_and_nothing_else(
    ) {
        // Older and newer majors.
        assert!(major_mismatch("v10.0.0", "v11.0.0-rc.10"));
        assert!(major_mismatch("v12.1.0", "v11.0.0"));
        assert!(major_mismatch("9.0.0", "v11.0.0"));
        // The same major, a newer minor included: not a major mismatch, and the
        // strict check still refuses the newer minor.
        assert!(!major_mismatch("v11.0.0", "v11.0.0-rc.10"));
        assert!(!major_mismatch("v11.2.0", "v11.0.0"));
        assert!(check_version_compatibility("v11.2.0", "v11.0.0").is_err());
        // `dev` and unreadable versions are not major mismatches.
        assert!(!major_mismatch("dev", "v11.0.0"));
        assert!(!major_mismatch("v10.0.0", "dev"));
        assert!(!major_mismatch("not-a-version", "v11.0.0"));
        assert!(!major_mismatch("v10.0.0", "not-a-version"));
        // Malformed numeric parts the permissive parser would read as a major
        // are not a major mismatch either, so they stay refused.
        for malformed in [
            "v12.bad.0",
            "12.0.x",
            "v12..0",
            "v12.0.0.0",
            "v-12.0.0",
            "12a.0.0",
            "",
        ] {
            assert!(!major_mismatch(malformed, "v11.0.0"), "{malformed}");
        }
        // Well-formed forms still classify: a bare major, a pre-release suffix,
        // build metadata alone or after a pre-release, on either side.
        assert!(major_mismatch("12", "v11.0.0"));
        assert!(major_mismatch("V10.0.0-rc.53", "v11.0.0-rc.10"));
        assert!(major_mismatch("v12.0.0+build.7", "v11.0.0"));
        assert!(major_mismatch("v12.0.0-rc.1+build.7", "v11.0.0"));
        assert!(major_mismatch("v10.0.0", "v11.0.0+build.7"));
        assert!(major_mismatch("v10.0.0", "v11.0.0-rc.10+build.7"));
        assert!(!major_mismatch("v11.0.0+build.7", "v11.0.0-rc.10"));
        // And metadata does not let a malformed core through.
        assert!(!major_mismatch("v12.bad.0+build.7", "v11.0.0"));
        assert!(!major_mismatch("v12.0.x-rc.1+build.7", "v11.0.0"));
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
