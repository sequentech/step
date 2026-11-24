// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Receipt admission, distinct from PKCS#11 device attestation. A normal
//! binary has no software-device variant. A dedicated demo binary needs an
//! independently installed read-only manifest and an exact database marker;
//! neither a request field nor a stored receipt can select the provider.

use anyhow::Result;
use deadpool_postgres::Transaction;
use sequent_core::signing::SigningAction;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstalledDevice {
    YubiHsm,
    #[cfg(feature = "operational-signing-demo-fixture")]
    DisposableSoftHsm,
}

impl InstalledDevice {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::YubiHsm => "yubihsm",
            #[cfg(feature = "operational-signing-demo-fixture")]
            Self::DisposableSoftHsm => "softhsm-test",
        }
    }

    pub(super) fn admits(self, profile: Option<&str>) -> bool {
        Some(self.as_str()) == profile
    }
}

/// Verify the fields the action will actually execute, not merely the hash of
/// a stored string. A changed Post/country/requester with the old canonical
/// bytes/digest must not allow the same receipt to release a different intent.
pub(super) fn matches_canonical_request(row: &crate::postgres::signing::SigningRequestRow) -> bool {
    use sequent_core::signing::{sha256_hex, SigningPayload, SigningPayloadFields};
    let fields = SigningPayloadFields {
        tenant_id: row.tenant_id.to_string(),
        election_event_id: row.election_event_id.to_string(),
        request_id: row.id,
        action: row.action,
        election_id: row.election_id.map(|id| id.to_string()),
        area_id: row.area_id.map(|id| id.to_string()),
        subject: row.subject.clone(),
        config_revision: row.config_revision.clone(),
        rule_revision: row.rule_revision,
        requested_by: row.requested_by.clone(),
        created_at: row.created_at,
        expires_at: row.expires_at,
    };
    SigningPayload::new(fields).is_ok_and(|payload| {
        payload.code() == row.code
            && payload.canonical().is_ok_and(|canonical| {
                canonical == row.canonical_payload
                    && sha256_hex(canonical.as_bytes()) == row.payload_sha256
            })
    })
}

#[cfg(not(feature = "operational-signing-demo-fixture"))]
pub async fn installed_profile(
    _tx: &Transaction<'_>,
    _tenant: Uuid,
    _event: Uuid,
    _action: SigningAction,
) -> Result<Option<InstalledDevice>> {
    // This is intentionally a compile-time partition, not a fallback from
    // lookup failure. Ordinary installations never admit a software receipt.
    Ok(Some(InstalledDevice::YubiHsm))
}

#[cfg(feature = "operational-signing-demo-fixture")]
pub use demo::{installed_demo, installed_profile, DemoInstallation};

#[cfg(feature = "operational-signing-demo-fixture")]
mod demo {
    use super::*;
    use anyhow::{ensure, Context};
    use serde::Deserialize;
    use std::{fs, io::Read, path::Path, sync::OnceLock};

    const ROOT: &str = "/run/sequent-operational-demo-14990";

    #[derive(Clone, Copy, Debug)]
    pub struct DemoInstallation {
        pub tenant: Uuid,
        pub event: Uuid,
        pub run: Uuid,
        pub device: InstalledDevice,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Manifest {
        tenant: Uuid,
        event: Uuid,
        backend: String,
        token_label: String,
        token_serial_sha256: String,
    }

    fn read_immutable(dir: &Path, filename: &str) -> Result<Vec<u8>> {
        ensure!(
            dir.canonicalize()?.as_path() == dir,
            "unexpected installation mount"
        );
        let path = dir.join(filename);
        let meta = fs::symlink_metadata(&path)?;
        ensure!(
            meta.is_file() && (1..=4096).contains(&meta.len()),
            "bad installation input"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            ensure!(
                meta.permissions().mode() & 0o222 == 0,
                "installation input is writable"
            );
        }
        let mut bytes = Vec::new();
        fs::File::open(path)?.take(4097).read_to_end(&mut bytes)?;
        ensure!(
            !bytes.is_empty() && bytes.len() <= 4096,
            "bad installation size"
        );
        Ok(bytes)
    }

    impl DemoInstallation {
        fn parse(manifest: &[u8], run_marker: &[u8]) -> Result<Self> {
            let m: Manifest =
                serde_json::from_slice(manifest).context("invalid installation manifest")?;
            let run: Uuid = std::str::from_utf8(run_marker)?
                .trim_end_matches('\n')
                .parse()?;
            ensure!(
                !run.is_nil() && format!("{run}\n").as_bytes() == run_marker,
                "invalid installation ID"
            );
            ensure!(
                !m.tenant.is_nil()
                    && !m.event.is_nil()
                    && !m.token_label.is_empty()
                    && m.token_label.len() <= 32
                    && m.token_label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
                    && m.token_serial_sha256.len() == 64
                    && m.token_serial_sha256
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
                "invalid installed scope or token"
            );
            let device = match m.backend.as_str() {
                "yubihsm" => InstalledDevice::YubiHsm,
                "softhsm-test" => InstalledDevice::DisposableSoftHsm,
                _ => anyhow::bail!("unknown installed signing device"),
            };
            Ok(Self {
                tenant: m.tenant,
                event: m.event,
                run,
                device,
            })
        }

        fn admits_scope(self, tenant: Uuid, event: Uuid, action: SigningAction) -> bool {
            self.tenant == tenant
                && self.event == event
                && action == SigningAction::InitializeVoting
        }
    }

    /// Read once. Recovery from a missing/changed mount requires an operator
    /// restart; never switch devices or accept the ordinary fallback in-process.
    pub fn installed_demo() -> Option<DemoInstallation> {
        static INSTALL: OnceLock<Option<DemoInstallation>> = OnceLock::new();
        *INSTALL.get_or_init(|| {
            let root = Path::new(ROOT);
            DemoInstallation::parse(
                &read_immutable(root, "installation.json").ok()?,
                &read_immutable(root, "run-id").ok()?,
            )
            .ok()
        })
    }

    pub async fn installed_profile(
        tx: &Transaction<'_>,
        tenant: Uuid,
        event: Uuid,
        action: SigningAction,
    ) -> Result<Option<InstalledDevice>> {
        let Some(installed) = installed_demo().filter(|i| i.admits_scope(tenant, event, action))
        else {
            return Ok(None);
        };
        let marker: Option<Uuid> = tx
            .query_one(
                "SELECT sequent_backend.operational_signing_demo_fixture_id($1,$2,$3,$4)",
                &[
                    &tenant,
                    &event,
                    &action.to_string(),
                    &installed.device.as_str(),
                ],
            )
            .await?
            .try_get(0)?;
        Ok((marker == Some(installed.run)).then_some(installed.device))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json::json;
        #[test]
        fn installation_binds_exact_two_closed_profiles_and_one_scope() {
            let tenant = Uuid::from_u128(1);
            let event = Uuid::from_u128(2);
            let run = Uuid::from_u128(3);
            let fixture = |backend: &str| {
                json!({"tenant":tenant,"event":event,"backend":backend,
                "token_label":"synthetic","token_serial_sha256":"ab".repeat(32)})
            };
            let r = format!("{run}\n");
            for (name, wrong) in [("softhsm-test", "yubihsm"), ("yubihsm", "softhsm-test")] {
                let i = DemoInstallation::parse(
                    &serde_json::to_vec(&fixture(name)).unwrap(),
                    r.as_bytes(),
                )
                .unwrap();
                assert!(i.device.admits(Some(name)));
                assert!(!i.device.admits(Some(wrong)));
                assert!(!i.device.admits(None));
                assert!(i.admits_scope(tenant, event, SigningAction::InitializeVoting));
                assert!(!i.admits_scope(tenant, run, SigningAction::InitializeVoting));
                assert!(!i.admits_scope(run, event, SigningAction::InitializeVoting));
                assert!(!i.admits_scope(tenant, event, SigningAction::TransmitResults));
            }
            let mut bad = fixture("softhsm-test");
            bad["hardware"] = true.into();
            assert!(
                DemoInstallation::parse(&serde_json::to_vec(&bad).unwrap(), r.as_bytes()).is_err()
            );
            assert!(DemoInstallation::parse(
                &serde_json::to_vec(&fixture("normal")).unwrap(),
                r.as_bytes()
            )
            .is_err());
        }
    }
}

// Test the consumption-time reconstruction with the actual typed row. Neither
// the stored canonical bytes nor their digest change in the tamper cases.
#[cfg(test)]
mod canonical_tests {
    use super::*;
    use crate::postgres::signing::SigningRequestRow;
    use chrono::{TimeZone, Utc};
    use sequent_core::signing::{
        sha256_hex, SigningPayload, SigningPayloadFields, SigningRequestStatus,
    };
    use serde_json::json;

    #[test]
    fn applied_receipt_is_bound_to_the_executed_post_country_subject_and_requester() {
        let id = Uuid::from_u128;
        let at = Utc.timestamp_opt(1_700_000_000, 0).single().unwrap();
        let fields = SigningPayloadFields {
            tenant_id: id(1).to_string(),
            election_event_id: id(2).to_string(),
            request_id: id(3),
            action: SigningAction::InitializeVoting,
            election_id: Some(id(4).to_string()),
            area_id: Some(id(5).to_string()),
            subject: json!({"publication_id":id(6)}),
            config_revision: None,
            rule_revision: 9,
            requested_by: id(7).to_string(),
            created_at: at,
            expires_at: Some(at + chrono::Duration::minutes(10)),
        };
        let signed = SigningPayload::new(fields.clone()).unwrap();
        let canonical = signed.canonical().unwrap();
        let row = SigningRequestRow {
            id: fields.request_id,
            tenant_id: id(1),
            election_event_id: id(2),
            action: fields.action,
            election_id: Some(id(4)),
            area_id: Some(id(5)),
            trustee_id: None,
            scope_key: "test-only".into(),
            subject: fields.subject,
            payload_sha256: sha256_hex(canonical.as_bytes()),
            canonical_payload: canonical,
            code: signed.code().into(),
            document_id: None,
            document_sha256: None,
            config_revision: None,
            rule_revision: fields.rule_revision,
            rule_snapshot: json!({}),
            required: 2,
            status: SigningRequestStatus::Waiting,
            cancel_reason: None,
            cancelled_by: None,
            requested_by: fields.requested_by,
            requested_by_username: "test".into(),
            requested_by_name: None,
            expires_at: fields.expires_at,
            completed_at: None,
            executed_at: None,
            execution_result: None,
            task_execution_id: None,
            execution_started_at: None,
            execution_attempts: 0,
            open_failures: 0,
            permission_label: None,
            created_at: at,
        };
        assert!(matches_canonical_request(&row));
        for changed in [
            SigningRequestRow {
                election_id: Some(id(8)),
                ..row.clone()
            },
            SigningRequestRow {
                area_id: None,
                ..row.clone()
            },
            SigningRequestRow {
                subject: json!({"publication_id":id(8)}),
                ..row.clone()
            },
            SigningRequestRow {
                requested_by: id(8).to_string(),
                ..row.clone()
            },
            SigningRequestRow {
                action: SigningAction::OpenVoting,
                ..row.clone()
            },
            SigningRequestRow {
                rule_revision: 10,
                ..row.clone()
            },
            SigningRequestRow {
                tenant_id: id(8),
                ..row.clone()
            },
            SigningRequestRow {
                code: "00000000".into(),
                ..row
            },
        ] {
            assert!(!matches_canonical_request(&changed));
        }
    }
}

// The independent socket-only witness runs via test-canonical-reload.sh.
// The ordinary candidate CI runs the real guard/commit/reload assertions in
// postgres_signing_operational_initialize using its own migrated fixture.
#[cfg(all(test, feature = "operational-signing-demo-fixture"))]
#[path = "operational_device_postgres_tests.rs"]
mod canonical_postgres_tests;
