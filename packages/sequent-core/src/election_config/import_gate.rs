// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The version, schema and validation checks the platform's importer runs on
//! a bundle, in its order, for tests.
//!
//! Mirrors part of `get_election_event_schema` and `check_bundle` in
//! windmill's `services/import/import_election_event.rs`:
//!
//! 1. parse the member as JSON;
//! 2. adopt legacy field names;
//! 3. read `version` off the raw JSON, a missing or non-string value meaning
//!    [`HISTORICAL_DEFAULT_VERSION`], and check it against the platform's
//!    `APP_VERSION` ([`check_version_compatibility`]): a different **major**
//!    ([`major_mismatch`]) is a warning and the import goes on (meta#14760);
//!    anything else that check refuses still refuses;
//! 4. deserialize into [`ImportElectionEventSchema`];
//! 5. refuse a bundle [`validate`] finds errors in.
//!
//! Not mirrored: windmill's masked-integration-secret refusal and
//! `replace_ids`, which follow these steps, and everything after them that
//! needs the database. Passing here is therefore not a tenant import; it shows
//! the bundle is not refused on its version, its shape or its validation.
//!
//! Tests that only deserialized and validated a generated bundle passed while
//! every v11 platform refused it at step 3, so these tests check the member
//! against literal platform versions.

use super::archive::{zip, Artifact};
use super::import_problems::version_major_mismatch;
use super::legacy_names::adopt_legacy_names;
use super::problem::Problem;
use super::{validate, ImportElectionEventSchema, ValidationReport};
use crate::util::version::{
    check_version_compatibility, major_mismatch, HISTORICAL_DEFAULT_VERSION,
    VERSION_KEY,
};
use serde_json::Value;

/// The OVCS release the COMELEC Architect's downloads are imported into.
pub(crate) const DEPLOYED_OVCS: &str = "v11.0.0-rc.10";

/// Where the gate stopped.
#[derive(Debug)]
pub(crate) enum Refused {
    NotJson(String),
    Version(String),
    NotABundle(String),
    Invalid(String),
}

impl Refused {
    /// What the importer would have said, for a failing test's message.
    pub(crate) fn reason(&self) -> &str {
        match self {
            Refused::NotJson(why)
            | Refused::Version(why)
            | Refused::NotABundle(why)
            | Refused::Invalid(why) => why,
        }
    }
}

/// What the gate let through, and what it warned about on the way.
#[derive(Debug)]
pub(crate) struct Admitted {
    pub(crate) schema: ImportElectionEventSchema,
    pub(crate) warnings: Vec<Problem>,
}

impl Admitted {
    /// The warning a different major raised, if it did.
    pub(crate) fn major_mismatch(&self) -> Option<&Problem> {
        self.warnings.iter().find(|problem| {
            problem.id.as_deref() == Some("file.version-major-mismatch")
        })
    }
}

/// Runs the gate on an `export_election_event-*.json` member's bytes, as
/// imported into a platform whose `APP_VERSION` is `platform`.
pub(crate) fn admit(
    member: &[u8],
    platform: &str,
) -> Result<Admitted, Refused> {
    let mut raw: Value = serde_json::from_slice(member)
        .map_err(|error| Refused::NotJson(error.to_string()))?;
    let mut legacy = ValidationReport::default();
    adopt_legacy_names(&mut raw, &mut legacy);
    let imported = raw
        .get(VERSION_KEY)
        .and_then(Value::as_str)
        .unwrap_or(HISTORICAL_DEFAULT_VERSION)
        .to_string();
    let mut warnings = Vec::new();
    if let Err(error) = check_version_compatibility(&imported, platform) {
        if major_mismatch(&imported, platform) {
            warnings.push(version_major_mismatch(&imported, platform));
        } else {
            return Err(Refused::Version(error.to_string()));
        }
    }
    let schema: ImportElectionEventSchema = serde_json::from_value(raw)
        .map_err(|error| Refused::NotABundle(error.to_string()))?;
    let report = validate(&schema);
    if report.has_errors() {
        return Err(Refused::Invalid(report.to_string()));
    }
    Ok(Admitted { schema, warnings })
}

/// The `export_election_event-*.json` member read back out of the importable
/// zip's actual bytes, as written by [`zip`], so the test sees what an
/// operator uploads rather than the in-memory layout.
pub(crate) fn event_member(importable: &[Artifact]) -> Vec<u8> {
    use std::io::Read;
    let bytes = zip(importable).expect("the importable members zip");
    let mut archive = ::zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .expect("the importable zip reads back");
    let names: Vec<String> = archive
        .file_names()
        .filter(|name| {
            name.starts_with("export_election_event-")
                && name.ends_with(".json")
        })
        .map(str::to_string)
        .collect();
    assert_eq!(
        names.len(),
        1,
        "exactly one election event member: {names:?}"
    );
    let mut member = Vec::new();
    archive
        .by_name(&names[0])
        .expect("the member is in the zip")
        .read_to_end(&mut member)
        .expect("the member reads");
    member
}

/// What the Architect's bundle must satisfy: every platform on its own line
/// admits it with **no** version warning, and a platform of another major
/// (v10 here) admits it too, with the major-mismatch warning (meta#14760).
pub(crate) fn assert_v11_admits_cleanly_and_v10_warns(member: &[u8]) {
    for platform in [DEPLOYED_OVCS, "v11.0.0"] {
        match admit(member, platform) {
            Ok(admitted) => assert!(
                admitted.warnings.is_empty(),
                "a {platform} platform warns about the bundle: {:?}",
                admitted.warnings
            ),
            Err(refused) => panic!(
                "a {platform} platform refuses the bundle: {}",
                refused.reason()
            ),
        }
    }
    match admit(member, "v10.0.0") {
        Ok(admitted) => {
            let warning = admitted
                .major_mismatch()
                .expect("a v10.0.0 platform warns about a v11 bundle");
            assert!(warning.message.contains("may not be imported"));
        }
        Err(refused) => panic!(
            "a v10.0.0 platform must import a v11 bundle with a warning, \
             not refuse it: {}",
            refused.reason()
        ),
    }
}

#[cfg(test)]
mod tests {
    //! The import's version policy (meta#14760), case by case, on synthetic
    //! bundles: a different major warns; nothing else the version check
    //! refused is relaxed, and nothing after it is.
    use super::*;
    use serde_json::json;

    /// The shared sound bundle (it deserializes and validates), with `version`.
    fn bundle(version: Option<&str>) -> Vec<u8> {
        let mut document: Value =
            serde_json::from_str(crate::election_config::fixtures::SOUND)
                .unwrap();
        match version {
            Some(version) => document["version"] = json!(version),
            None => {
                document.as_object_mut().unwrap().remove("version");
            }
        }
        serde_json::to_vec(&document).unwrap()
    }

    #[test]
    fn an_older_or_newer_major_is_imported_with_a_warning() {
        for (found, platform) in
            [("v10.0.0", DEPLOYED_OVCS), ("v12.0.0", "v11.0.0")]
        {
            let admitted = admit(&bundle(Some(found)), platform)
                .unwrap_or_else(|refused| panic!("{}", refused.reason()));
            let warning = admitted.major_mismatch().expect("a warning");
            assert_eq!(
                warning.severity,
                crate::election_config::Severity::Warning
            );
            assert_eq!(
                warning.details.get("found").map(String::as_str),
                Some(found)
            );
            assert!(!warning.message.contains("compatible"));
        }
        // No version at all reads as the historical 9.0.0: also a different
        // major, also a warning.
        let admitted = admit(&bundle(None), DEPLOYED_OVCS).unwrap();
        assert!(admitted.major_mismatch().is_some());
    }

    #[test]
    fn the_same_major_imports_without_a_warning() {
        let admitted = admit(&bundle(Some("v11.0.0")), DEPLOYED_OVCS).unwrap();
        assert!(admitted.warnings.is_empty());
    }

    #[test]
    fn what_is_not_a_major_mismatch_still_refuses() {
        for (found, platform) in [
            // A newer minor of the same major.
            ("v11.2.0", "v11.0.0"),
            // `dev` on one side only.
            ("dev", DEPLOYED_OVCS),
            // An unreadable version, and malformed numeric parts.
            ("not-a-version", DEPLOYED_OVCS),
            ("v12.bad.0", DEPLOYED_OVCS),
        ] {
            assert!(
                matches!(
                    admit(&bundle(Some(found)), platform),
                    Err(Refused::Version(_))
                ),
                "{found} on {platform} must still be refused"
            );
        }
        // `dev` on both sides is the strict check's own exception.
        assert!(admit(&bundle(Some("dev")), "dev").is_ok());
    }

    #[test]
    fn a_major_mismatch_does_not_excuse_a_real_schema_or_validation_error() {
        let mut broken: Value =
            serde_json::from_slice(&bundle(Some("v10.0.0"))).unwrap();
        broken["elections"] = json!("not a list");
        assert!(matches!(
            admit(&serde_json::to_vec(&broken).unwrap(), DEPLOYED_OVCS),
            Err(Refused::NotABundle(_))
        ));
        let mut invalid: Value =
            serde_json::from_slice(&bundle(Some("v10.0.0"))).unwrap();
        invalid["signing_rules"] = json!([
            {"action": "open-voting", "requirement": "required", "signatures": 0,
             "requester_signing": "allowed"}
        ]);
        assert!(matches!(
            admit(&serde_json::to_vec(&invalid).unwrap(), DEPLOYED_OVCS),
            Err(Refused::Invalid(_))
        ));
    }
}
