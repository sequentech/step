// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::configuration_package_sections::AdminRow;
use super::import_users::{GroupAssignmentPolicy, ImportUsersPrivileges};
use crate::services::browser_package_policy::BrowserPolicy;
use anyhow::{anyhow, Result};
use sequent_core::election_config::{
    archive::Artifact,
    demo_credentials::{
        default_demo_login_roster, verify_demo_login_roster, verify_demo_login_verifiers,
        DemoLoginPrincipal, DemoLoginVerifiers, DEMO_LOGIN_VERIFIERS_MEMBER,
    },
    design::DesignDigestAlgorithm,
    manifest::ManifestFormat,
    package_verify::{VerificationScheme, VerifiedPackage},
};
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::keycloak::{User, TENANT_ID_ATTR_NAME};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct InstalledPolicy {
    tenant_id: String,
    accounts: Vec<DemoLoginPrincipal>,
}

fn receiver_policy(tenant_id: &str) -> Result<InstalledPolicy> {
    match std::env::var("SEQUENT_DEMO_LOGIN_RECEIVER_POLICY") {
        Err(std::env::VarError::NotPresent) => Ok(InstalledPolicy {
            tenant_id: tenant_id.to_owned(),
            accounts: default_demo_login_roster(),
        }),
        Ok(value) if value.len() <= 64 * 1024 => serde_json::from_str::<InstalledPolicy>(&value)
            .ok()
            .filter(|policy| {
                policy.tenant_id == tenant_id && verify_demo_login_roster(&policy.accounts).is_ok()
            })
            .ok_or_else(|| anyhow!("no valid installed workbook login policy for this tenant")),
        _ => Err(anyhow!(
            "no valid installed workbook login policy for this tenant"
        )),
    }
}

pub struct VerifiedDemoLoginAdmission(DemoLoginVerifiers);

impl std::fmt::Debug for VerifiedDemoLoginAdmission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("VerifiedDemoLoginAdmission(<private>)")
    }
}

fn field<'a>(row: &'a AdminRow, key: &str) -> Option<&'a str> {
    row.fields
        .get(key)
        .filter(|value| !value.is_empty())
        .map(String::as_str)
}

/// A browser delivery carries no certificate tenant binding. Its independent
/// proof must have verified against this deployment's active managed authority.
fn verified_for_tenant(proof: &VerifiedPackage, policy: &BrowserPolicy, tenant_id: &str) -> bool {
    match (&proof.scheme, policy) {
        (VerificationScheme::Certificate, BrowserPolicy::Off | BrowserPolicy::OtherTenant) => {
            proof.manifest.format == ManifestFormat::V2
                && proof
                    .manifest
                    .signing
                    .as_ref()
                    .is_some_and(|s| s.tenant_id == tenant_id)
        }
        (
            VerificationScheme::Browser { roster_sha256 },
            BrowserPolicy::ManagedOnly(pinned, binding),
        ) => {
            proof.manifest.format == ManifestFormat::V1
                && proof.manifest.signing.is_none()
                && proof.manifest.content.ballot_design_digest_algorithm
                    == DesignDigestAlgorithm::V2
                && pinned.tenant() == tenant_id
                && pinned.sha256() == roster_sha256
                && binding.sha256.as_str() == roster_sha256
                && binding.required_approvals == pinned.required()
        }
        _ => false,
    }
}

impl VerifiedDemoLoginAdmission {
    /// Proof comes from server-side signature/target admission, not a digest
    /// or transport marker supplied by the client. Default is deny.
    pub(crate) fn take_member(
        members: &mut Vec<Artifact>,
        verified: Option<&VerifiedPackage>,
        tenant_id: &str,
        policy: &BrowserPolicy,
        privileges: ImportUsersPrivileges,
    ) -> Result<Option<Self>> {
        let entries = members
            .iter()
            .enumerate()
            .filter(|(_, f)| f.name == DEMO_LOGIN_VERIFIERS_MEMBER);
        let mut entries = entries.peekable();
        let Some((index, file)) = entries.next() else {
            return Ok(None);
        };
        if entries.peek().is_some()
            || file.bytes.len() > 128 * 1024
            || privileges.group_assignment != GroupAssignmentPolicy::AnyGroup
            || !verified.is_some_and(|proof| {
                verified_for_tenant(proof, policy, tenant_id)
                    && proof
                        .members
                        .iter()
                        .any(|m| m.name == DEMO_LOGIN_VERIFIERS_MEMBER && m.bytes == file.bytes)
            })
        {
            return Err(anyhow!("demo account enrollment requires a verified tenant-bound signature and user/role write permission"));
        }
        let policy = match std::env::var("SEQUENT_WORKBOOK_PASSWORDS") {
            Err(std::env::VarError::NotPresent) => receiver_policy(tenant_id)?,
            Ok(value) if value == "on" => receiver_policy(tenant_id)?,
            _ => {
                return Err(anyhow!(
                    "workbook password enrollment is disabled by the operator"
                ))
            }
        };
        let payload: DemoLoginVerifiers = serde_json::from_slice(&file.bytes)
            .map_err(|_| anyhow!("malformed signed demo login verifier document"))?;
        verify_demo_login_verifiers(&payload,tenant_id,&policy.accounts)
            .map_err(|_|anyhow!("signed demo login verifier does not match approved roster or supported hashing scheme"))?;
        members.remove(index);
        Ok(Some(Self(payload)))
    }

    pub(crate) fn verify_roster(&self, rows: &[AdminRow]) -> Result<()> {
        for a in &self.0.accounts {
            let matched = rows.iter().any(|r| {
                r.username == a.username
                    && r.group == a.group_name
                    && field(r, "email") == a.email.as_deref()
                    && field(r, "trustee") == a.trustee.as_deref()
            });
            if !matched {
                return Err(anyhow!(
                    "signed demo verifier does not match signed administrator roster identity"
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn new_usernames(
        &self,
        existing: &BTreeMap<String, BTreeSet<String>>,
    ) -> Vec<String> {
        self.0
            .accounts
            .iter()
            .filter(|a| !existing.contains_key(&a.username))
            .map(|a| a.username.clone())
            .collect()
    }

    /// Only actual new-account transitions need the additional live role
    /// recheck. Repeating a signed package never resets existing accounts.
    pub(crate) async fn authorized_new_usernames<F, Fut>(
        &self,
        existing: &BTreeMap<String, BTreeSet<String>>,
        require_live_editor: F,
    ) -> Result<Vec<String>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let names = self.new_usernames(existing);
        if !names.is_empty() {
            require_live_editor().await?;
        }
        Ok(names)
    }

    pub(crate) async fn seed_new(
        &self,
        tenant: &str,
        realm: &str,
        rows: &[AdminRow],
        existing: &BTreeMap<String, BTreeSet<String>>,
        actor: Option<&crate::services::audit_log::AuditActor>,
    ) -> Result<Vec<String>> {
        self.seed_new_with_audit(
            tenant,
            realm,
            rows,
            existing,
            actor,
            crate::services::task_audit::record,
        )
        .await
    }

    /// The application always writes through the production audit sink. The
    /// isolated Keycloak contract test captures those same audit changes locally.
    async fn seed_new_with_audit<F, Fut>(
        &self,
        tenant: &str,
        realm: &str,
        rows: &[AdminRow],
        existing: &BTreeMap<String, BTreeSet<String>>,
        actor: Option<&crate::services::audit_log::AuditActor>,
        mut record: F,
    ) -> Result<Vec<String>>
    where
        F: FnMut(crate::services::audit_log::AdminChange) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        self.verify_roster(rows)?;
        let names = self.new_usernames(existing);
        if names.is_empty() {
            return Ok(names);
        }
        let actor = actor
            .filter(|a| a.user_id.as_deref().is_some_and(|id| !id.is_empty()))
            .ok_or_else(|| {
                anyhow!("new demo account enrollment requires an attributable operator")
            })?;
        let client = KeycloakAdminClient::new()
            .await
            .map_err(|_| anyhow!("could not open target identity store"))?;
        for a in self
            .0
            .accounts
            .iter()
            .filter(|a| !existing.contains_key(&a.username))
        {
            let r = rows
                .iter()
                .find(|r| r.username == a.username)
                .ok_or_else(|| anyhow!("signed demo user is absent from roster"))?;
            let mut attrs =
                HashMap::from([(TENANT_ID_ATTR_NAME.to_owned(), vec![tenant.to_owned()])]);
            for key in [
                "permission_labels",
                "trustee",
                "title",
                "sequent.read-only.mobile-number",
            ] {
                if let Some(v) = field(r, key) {
                    attrs.insert(key.to_owned(), v.split('|').map(str::to_owned).collect());
                }
            }
            let user = User {
                username: Some(r.username.clone()),
                email: field(r, "email").map(str::to_owned),
                first_name: field(r, "first_name").map(str::to_owned),
                last_name: field(r, "last_name").map(str::to_owned),
                email_verified: Some(field(r, "email_verified") != Some("false")),
                enabled: Some(field(r, "enabled") != Some("false")),
                attributes: Some(attrs),
                ..Default::default()
            };
            // Only POST new identity. A conflict is never retried as an update.
            client
                .create_user_with_demo_password_verifier(realm, &user, &r.group, a)
                .await
                .map_err(|_| {
                    anyhow!("target identity store refused a new demo account; preview again")
                })?;
            // Record each successful creation immediately, even if a later
            // account fails. Never include the email, password or verifier.
            use crate::services::{audit_log::AdminAction, task_audit};
            record(task_audit::bulk(
                Some(actor.clone()),
                task_audit::target(tenant, None)?,
                "user",
                AdminAction::Create,
                "Created an allowlisted demo user from a verified configuration package",
                serde_json::json!({"username": r.username, "group": r.group}),
            ))
            .await?;
        }
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_even_with_named_member_has_no_credential_admission() {
        let mut files = vec![Artifact {
            name: DEMO_LOGIN_VERIFIERS_MEMBER.into(),
            bytes: b"dummy-not-logged".to_vec(),
        }];
        let permitted = ImportUsersPrivileges {
            group_assignment: GroupAssignmentPolicy::AnyGroup,
            ..Default::default()
        };
        let refused = VerifiedDemoLoginAdmission::take_member(
            &mut files,
            None,
            "unused",
            &BrowserPolicy::Off,
            permitted,
        )
        .unwrap_err();
        assert!(!refused.to_string().contains("dummy-not-logged"));
        assert_eq!(files.len(), 1);
    }

    #[tokio::test]
    async fn credential_seeding_rechecks_a_revoked_editor_but_never_resets_existing_users() {
        let payload: DemoLoginVerifiers = serde_json::from_value(serde_json::json!({
            "format":"sequent.demo-login-verifiers/1","tenantId":"synthetic",
            "accounts":[{"username":"trustee1","email":null,"groupName":"trustee","trustee":"trustee1",
                "iterations":600000,"salt":"fixture-salt","verifier":"fixture-verifier"}]
        })).unwrap();
        let admission = VerifiedDemoLoginAdmission(payload);
        let absent = BTreeMap::new();
        assert!(admission
            .authorized_new_usernames(&absent, || async {
                Err(anyhow!("editor's privileges were revoked"))
            })
            .await
            .is_err());
        assert_eq!(
            admission
                .authorized_new_usernames(&absent, || async { Ok(()) })
                .await
                .unwrap(),
            ["trustee1"]
        );
        let exists = BTreeMap::from([("trustee1".into(), BTreeSet::new())]);
        assert!(admission
            .authorized_new_usernames(&exists, || async {
                panic!("existing identities never get credential writes")
            })
            .await
            .unwrap()
            .is_empty());
    }

    #[test]
    fn qualified_roster_can_only_seed_new_accounts_and_exact_identity() {
        let payload: DemoLoginVerifiers = serde_json::from_value(serde_json::json!({
            "format":"sequent.demo-login-verifiers/1","tenantId":"synthetic",
            "accounts":[{"username":"trustee1","email":null,"groupName":"trustee","trustee":"trustee1",
                "iterations":600000,"salt":"fixture-salt","verifier":"fixture-verifier"}]
        })).unwrap();
        let admission = VerifiedDemoLoginAdmission(payload);
        let mut row = AdminRow {
            username: "trustee1".into(),
            group: "trustee".into(),
            fields: BTreeMap::from([("trustee".into(), "trustee1".into())]),
        };
        assert!(admission.verify_roster(std::slice::from_ref(&row)).is_ok());
        assert_eq!(admission.new_usernames(&BTreeMap::new()), vec!["trustee1"]);
        assert!(admission
            .new_usernames(&BTreeMap::from([("trustee1".into(), BTreeSet::new())]))
            .is_empty());
        row.fields
            .insert("email".into(), "other@example.invalid".into());
        assert!(admission.verify_roster(&[row]).is_err());
        assert!(!format!("{admission:?}").contains("fixture-verifier"));
    }
}

#[cfg(test)]
#[path = "../../../tests/support/managed_workbook_credentials_tests.rs"]
mod browser_tests;
