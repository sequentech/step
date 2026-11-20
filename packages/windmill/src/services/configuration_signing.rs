// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Setting what a tenant trusts in configuration packages: its
//! `settings.configuration_signing`. This is the one way to write it; the
//! database refuses every other write of that key, and package imports and
//! restored backups keep the installed value.

use anyhow::Result;
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::election_config::package_verify::{
    ConfigurationSigning, ConfigurationSigningPolicy, TrustCheck,
};
use sequent_core::election_config::signing_policy::{
    EnrolledSigner, FinalSignerSeparation, PackageFormat,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::postgres::configuration_packages::try_lock_package_work;
use crate::postgres::tenant::write_configuration_signing;

/// The setting as an administrator submits it. Unknown fields are refused,
/// so a misspelled one is never silently dropped from what the tenant
/// trusts.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationSigningInput {
    pub policy: ConfigurationSigningPolicy,
    #[serde(default)]
    pub package_roots: String,
    #[serde(default)]
    pub staff_roots: String,
    #[serde(default)]
    pub required_approvals: u16,
    /// Format 2: the oldest package format the tenant imports. Required
    /// with signers (checked by the core policy check).
    #[serde(default)]
    pub minimum_package_format: Option<PackageFormat>,
    /// Format 2: enrolled signers with stable IDs, roles and key pins.
    #[serde(default)]
    pub signers: Vec<EnrolledSigner>,
    /// Format 2: whether the final signer may also have approved.
    #[serde(default)]
    pub final_signer_separation: Option<FinalSignerSeparation>,
}

impl From<ConfigurationSigningInput> for ConfigurationSigning {
    fn from(input: ConfigurationSigningInput) -> Self {
        ConfigurationSigning {
            policy: input.policy,
            package_roots: input.package_roots,
            staff_roots: input.staff_roots,
            required_approvals: input.required_approvals,
            minimum_package_format: input.minimum_package_format,
            signers: input.signers,
            final_signer_separation: input.final_signer_separation,
        }
    }
}

/// What saving did.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum SaveOutcome {
    /// Saved; `previous` is the value it replaced, if any.
    Saved {
        check: TrustCheck,
        previous: Option<Value>,
        saved: Value,
    },
    /// Not saved: the check found errors, and nothing was written.
    Refused { check: TrustCheck },
    /// Not saved: a package application or import is running on the tenant,
    /// and its trust doesn't change under it. Nothing was written.
    Busy,
}

/// Reads the submitted setting, refusing anything that is not one.
pub fn read_input(value: Value) -> Result<ConfigurationSigning, String> {
    serde_json::from_value::<ConfigurationSigningInput>(value)
        .map(ConfigurationSigning::from)
        .map_err(|error| format!("the configuration signing setting can't be read: {error}"))
}

/// Checks `setting` at `now` and, when it has no errors, writes it as the
/// tenant's within `hasura_transaction`. The caller commits.
pub async fn save(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    setting: &ConfigurationSigning,
    now: DateTime<Utc>,
) -> Result<SaveOutcome> {
    let check = setting.check(now);
    if check.report.has_errors() {
        return Ok(SaveOutcome::Refused { check });
    }
    // What is stored is the checked certificates alone, never other text
    // that came with them.
    let canonical = setting
        .canonical()
        .map_err(|problem| anyhow::anyhow!(problem.message))?;
    if !try_lock_package_work(hasura_transaction, tenant_id).await? {
        return Ok(SaveOutcome::Busy);
    }
    let saved = serde_json::to_value(&canonical)?;
    let previous = write_configuration_signing(hasura_transaction, tenant_id, &saved).await?;
    Ok(SaveOutcome::Saved {
        check,
        previous,
        saved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_complete_setting_reads() {
        let setting = read_input(json!({
            "policy": "required",
            "package_roots": "PEM",
            "staff_roots": "PEM",
            "required_approvals": 2
        }))
        .unwrap();
        assert_eq!(setting.policy, ConfigurationSigningPolicy::Required);
        assert_eq!(setting.required_approvals, 2);
    }

    #[test]
    fn an_unknown_field_or_policy_is_refused_rather_than_dropped() {
        assert!(
            read_input(json!({"policy": "required", "required_aprovals": 2}))
                .unwrap_err()
                .contains("required_aprovals")
        );
        assert!(read_input(json!({"policy": "sometimes"})).is_err());
        assert!(read_input(json!({"package_roots": "PEM"})).is_err());
        assert!(read_input(json!({"policy": "required", "required_approvals": -1})).is_err());
    }
}
