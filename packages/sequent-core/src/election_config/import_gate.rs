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
//!    [`HISTORICAL_DEFAULT_VERSION`], and refuse it on a major mismatch with
//!    the platform's `APP_VERSION` ([`check_version_compatibility`]);
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
use super::legacy_names::adopt_legacy_names;
use super::{validate, ImportElectionEventSchema, ValidationReport};
use crate::util::version::{
    check_version_compatibility, HISTORICAL_DEFAULT_VERSION, VERSION_KEY,
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

/// Runs the gate on an `export_election_event-*.json` member's bytes, as
/// imported into a platform whose `APP_VERSION` is `platform`.
pub(crate) fn admit(
    member: &[u8],
    platform: &str,
) -> Result<ImportElectionEventSchema, Refused> {
    let mut raw: Value = serde_json::from_slice(member)
        .map_err(|error| Refused::NotJson(error.to_string()))?;
    let mut legacy = ValidationReport::default();
    adopt_legacy_names(&mut raw, &mut legacy);
    let imported = raw
        .get(VERSION_KEY)
        .and_then(Value::as_str)
        .unwrap_or(HISTORICAL_DEFAULT_VERSION)
        .to_string();
    check_version_compatibility(&imported, platform)
        .map_err(|error| Refused::Version(error.to_string()))?;
    let schema: ImportElectionEventSchema = serde_json::from_value(raw)
        .map_err(|error| Refused::NotABundle(error.to_string()))?;
    let report = validate(&schema);
    if report.has_errors() {
        return Err(Refused::Invalid(report.to_string()));
    }
    Ok(schema)
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

/// What the default bundle must satisfy on every platform this crate's line
/// covers, and fail on the line before it.
pub(crate) fn assert_v11_admits_and_v10_refuses(member: &[u8]) {
    for platform in [DEPLOYED_OVCS, "v11.0.0"] {
        if let Err(refused) = admit(member, platform) {
            panic!(
                "a {platform} platform refuses the bundle: {}",
                refused.reason()
            );
        }
    }
    match admit(member, "v10.0.0") {
        Err(Refused::Version(message)) => {
            assert!(message.contains("Version mismatch"), "{message}")
        }
        other => panic!(
            "a v10.0.0 platform must refuse a v11 bundle on its version, \
             got {:?}",
            other.map(|_| "admitted")
        ),
    }
}
