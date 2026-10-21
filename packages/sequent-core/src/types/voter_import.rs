// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Integrity requirements and completed voter-import evidence shared by the
//! importer, the portals and readiness verification.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[allow(non_camel_case_types)]
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
)]
pub enum FileIntegrityPolicy {
    #[default]
    OPTIONAL,
    REQUIRED_SHA256,
}

impl FileIntegrityPolicy {
    /// Validate the declared digest before processing a file. A provided digest
    /// always has to be valid, including on legacy optional-policy events.
    pub fn validate_declared_sha256(
        self,
        declared: Option<&str>,
    ) -> Result<Option<String>, String> {
        let declared =
            declared.map(str::trim).filter(|value| !value.is_empty());
        match declared {
            None if self == Self::REQUIRED_SHA256 => {
                Err("A certified file SHA-256 is required".into())
            }
            None => Ok(None),
            Some(value) if valid_sha256(value) => {
                Ok(Some(value.to_ascii_lowercase()))
            }
            Some(_) => {
                Err("SHA-256 must contain exactly 64 hexadecimal characters"
                    .into())
            }
        }
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// A successful import, persisted only after the voter transaction commits.
/// `import_id` is the audit outbox UUID and the signed audit entry's entry_id.
/// Area keys are stable area names, as in the signed configuration's design
/// keys. An empty key retains unassigned rows and cannot certify readiness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VotersImportedEvidence {
    pub import_id: String,
    pub document_id: String,
    pub file_name: String,
    pub sha256: String,
    pub declared_sha256: Option<String>,
    #[serde(alias = "counts_by_post")]
    pub counts_by_area: BTreeMap<String, u64>,
    pub imported_at: String,
}

impl VotersImportedEvidence {
    /// Readiness also checks these area keys against the verified manifest and
    /// rejects later incomplete imports; a computed digest alone is not a
    /// certified transmittal.
    pub fn is_certified(&self) -> bool {
        valid_sha256(&self.sha256)
            && self.declared_sha256.as_ref().is_some_and(|declared| {
                valid_sha256(declared)
                    && declared.eq_ignore_ascii_case(&self.sha256)
            })
            && !self.counts_by_area.is_empty()
            && self.counts_by_area.keys().all(|key| !key.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "default_features")]
    use crate::ballot::ElectionEventPresentation;

    #[test]
    fn legacy_voter_receipts_read_but_new_json_uses_area_counts() {
        let legacy = serde_json::json!({
            "import_id":"import-1", "document_id":"document-1", "file_name":"registry.csv",
            "sha256":"ab".repeat(32), "declared_sha256":"ab".repeat(32),
            "counts_by_post":{"Area A":2,"Area B":1}, "imported_at":"2026-10-14T10:00:00Z"
        });
        let receipt: VotersImportedEvidence =
            serde_json::from_value(legacy).unwrap();
        assert!(receipt.is_certified());
        let neutral = serde_json::to_value(&receipt).unwrap();
        assert_eq!(
            neutral["counts_by_area"],
            serde_json::json!({"Area A":2,"Area B":1})
        );
        assert!(neutral.get("counts_by_post").is_none());
        assert_eq!(
            serde_json::from_value::<VotersImportedEvidence>(neutral).unwrap(),
            receipt
        );
    }

    #[test]
    fn required_sha256_rejects_missing_and_malformed_declarations() {
        for value in [
            None,
            Some(""),
            Some(" "),
            Some("bad"),
            Some(&"z".repeat(64)),
        ] {
            assert!(FileIntegrityPolicy::REQUIRED_SHA256
                .validate_declared_sha256(value)
                .is_err());
        }
        assert_eq!(
            FileIntegrityPolicy::OPTIONAL
                .validate_declared_sha256(None)
                .unwrap(),
            None
        );
        assert!(FileIntegrityPolicy::OPTIONAL
            .validate_declared_sha256(Some("bad"))
            .is_err());
        assert_eq!(
            FileIntegrityPolicy::REQUIRED_SHA256
                .validate_declared_sha256(Some(&"AB".repeat(32)))
                .unwrap(),
            Some("ab".repeat(32))
        );
    }

    #[cfg(feature = "default_features")]
    #[test]
    fn integrity_policy_keeps_existing_ballot_style_bytes() {
        let legacy = ElectionEventPresentation::default();
        assert_eq!(
            legacy.file_integrity_policy(),
            FileIntegrityPolicy::OPTIONAL
        );
        // Adding an absent null field would change historical canonical JSON
        // ballot-design digests even though Borsh deliberately skips it.
        assert!(serde_json::to_value(&legacy)
            .unwrap()
            .get("file_integrity_policy")
            .is_none());
        let required = ElectionEventPresentation {
            file_integrity_policy: Some(FileIntegrityPolicy::REQUIRED_SHA256),
            ..legacy.clone()
        };
        assert_eq!(
            borsh::to_vec(&legacy).unwrap(),
            borsh::to_vec(&required).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&required).unwrap()["file_integrity_policy"],
            "REQUIRED_SHA256"
        );
    }

    #[test]
    fn certification_requires_verified_hash_and_stable_area_counts() {
        let mut evidence = VotersImportedEvidence {
            import_id: "import-1".into(),
            document_id: "document-1".into(),
            file_name: "voters.csv".into(),
            sha256: "ab".repeat(32),
            declared_sha256: Some("ab".repeat(32)),
            counts_by_area: [("Area A".into(), 2)].into(),
            imported_at: "2026-10-14T10:00:00Z".into(),
        };
        assert!(evidence.is_certified());
        evidence.declared_sha256 = None;
        assert!(!evidence.is_certified());
        evidence.declared_sha256 = Some("ab".repeat(32));
        evidence.counts_by_area = [("".into(), 2)].into();
        assert!(!evidence.is_certified());
    }
}
