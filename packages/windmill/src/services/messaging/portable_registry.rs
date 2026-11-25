// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Resolve signed package-local references to narrowly matched tenant accounts.
//! Planning and applying the registry never run a provider or read its secrets.

use super::config::account_summary;
use crate::postgres::messaging::{
    account_endpoint, insert_messaging_account_with_id, AccountSettings, MessagingAccount,
};
use anyhow::{anyhow, ensure, Context, Result};
use chrono::{TimeZone, Utc};
use deadpool_postgres::Transaction;
use sequent_core::election_config::portable_messaging::{
    PortableMessagingAccount, PortableMessagingAccounts, REF_PREFIX,
};
use sequent_core::types::messaging::{
    AccountCheck, AccountLimits, AccountSender, AccountSummary, ProviderApproval, ReadinessPolicy,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableRegistryBinding {
    pub account_id: Uuid,
    /// Absent for a fresh row. No credential values or callback keys are included.
    pub existing_fingerprint: Option<String>,
}

/// Target-dependent evidence from preview, not part of the authored package.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableRegistryPlan {
    pub tenant_id: Uuid,
    pub package_sha256: String,
    pub bindings: BTreeMap<String, PortableRegistryBinding>,
}

impl PortableRegistryPlan {
    /// Rewrite the account references only, never the rest of the message config.
    pub fn bind(&self, config: &Value) -> Result<Value> {
        let mut config = config.clone();
        let Some(channels) = config.get_mut("channels") else {
            return Ok(config);
        };
        for channel in channels
            .as_array_mut()
            .context("messaging channels must be an array")?
        {
            let account = channel
                .get_mut("account_id")
                .context("messaging channel has no account reference")?;
            let reference = account
                .as_str()
                .context("invalid messaging account reference")?;
            if reference.starts_with(REF_PREFIX) {
                let binding = self
                    .bindings
                    .get(reference)
                    .context("messaging configuration references an undeclared portable account")?;
                *account = Value::String(binding.account_id.to_string());
            }
        }
        Ok(config)
    }

    /// Safe UI description: identifiers and credential metadata are not shown.
    pub fn change_rows(&self, definitions: &PortableMessagingAccounts) -> Result<Vec<Value>> {
        definitions.accounts.iter().map(|definition| {
            let binding = self.bindings.get(&definition.reference_key())
                .context("missing portable messaging binding")?;
            let wanted = settings(definition)?;
            Ok(json!({
                "name": wanted.name, "channel": wanted.channel,
                "provider": wanted.sender.provider(),
                "action": if binding.existing_fingerprint.is_some() { "reuse" } else { "create" }
            }))
        }).collect()
    }

    pub fn warnings(&self) -> Vec<String> {
        if self
            .bindings
            .values()
            .any(|binding| binding.existing_fingerprint.is_none())
        {
            vec!["New portable messaging accounts are unchecked and not defaults. Configure credentials in the target and run the normal provider checks before enabling delivery.".into()]
        } else {
            vec![]
        }
    }
}

/// Public: shown to the operator through the portable-messaging blocker allowlist.
pub const CREDENTIALS_NOT_ACCEPTED: &str = "portable messaging requires AWS credentials this receiver cannot accept yet; deliver the accounts without credentials and set keys on this installation";

fn package_digest(digest: &str) -> Result<&str> {
    ensure!(
        digest.len() == 64
            && digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "invalid package digest for portable messaging"
    );
    Ok(digest)
}

fn account_id(tenant: Uuid, package_sha: &str, reference: &str) -> Uuid {
    let mut digest = Sha256::new();
    digest.update(b"step:portable-messaging-account:v1\0");
    digest.update(tenant.as_bytes());
    // The package digest is fixed width. Only the final reference is variable.
    digest.update(package_sha.as_bytes());
    digest.update(reference.as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest.finalize()[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x80; // Custom RFC 9562 UUIDv8.
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn normalized_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn sender_matches(a: &AccountSender, b: &AccountSender) -> bool {
    // Meta's identity only describes the number/Page, not the API address.
    // Never substitute a different destination for the account's credentials.
    a.identity() == b.identity()
        && account_endpoint(a).map(|url| url.trim_end_matches('/'))
            == account_endpoint(b).map(|url| url.trim_end_matches('/'))
}

fn safe_status(status: &AccountCheck) -> AccountCheck {
    let mut status = status.clone();
    // The URL can contain a webhook key; neither it nor voter history is
    // needed when validating an event's account bindings.
    status.details.callback = None;
    status.details.number_changes.clear();
    status
}

fn account_fingerprint(account: &MessagingAccount) -> Result<String> {
    // Account.credentials is metadata (credential names and replacement
    // timestamps), not the encrypted credential store. Hash only the
    // allowlisted fields; do not serialize the whole account or webhook key.
    let fingerprint = json!({
        "id": account.id,
        "tenant": account.tenant_id,
        "channel": account.channel,
        "provider": account.provider,
        "name": account.name,
        "sender": account.sender,
        "limits": account.limits,
        "provider_approval": account.provider_approval,
        "readiness": account.readiness,
        "status": safe_status(&account.status),
        "credential_metadata": account.credentials,
        "is_default": account.is_default,
        "created_at": account.created_at,
        "updated_at": account.updated_at,
    });
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&fingerprint)?)
    ))
}

fn settings(definition: &PortableMessagingAccount) -> Result<AccountSettings> {
    let sender = definition.account_sender();
    let channel = sender
        .provider()
        .channel()
        .context("portable messaging account has no supported provider channel")?;
    let settings = AccountSettings {
        channel,
        name: definition.name.clone(),
        sender,
        limits: AccountLimits::default(),
        provider_approval: ProviderApproval::default(),
        readiness: ReadinessPolicy::PROVIDER_CHECK,
        is_default: false,
    };
    settings.validate()?;
    Ok(settings)
}

fn plan_from_accounts(
    tenant: Uuid,
    package_sha: &str,
    definitions: &PortableMessagingAccounts,
    existing: &[MessagingAccount],
) -> Result<PortableRegistryPlan> {
    package_digest(package_sha)?;
    definitions.validate()?;
    // A committed AWS key pair must reach the vault before any account is
    // planned. Until this receiver takes the protected sidecar, refuse at
    // Preview and Apply rather than create the account on the workload role.
    ensure!(
        definitions
            .accounts
            .iter()
            .all(|definition| definition.credential_commitment.is_none()),
        CREDENTIALS_NOT_ACCEPTED
    );
    let mut desired: Vec<AccountSettings> = vec![];
    let mut bindings = BTreeMap::new();
    let mut bound_ids = BTreeSet::new();
    for definition in &definitions.accounts {
        let reference = definition.reference_key();
        let wanted = settings(definition)?;
        let name = normalized_name(&wanted.name);
        ensure!(
            !desired
                .iter()
                .any(|previous| normalized_name(&previous.name) == name
                    || (previous.channel == wanted.channel
                        && previous.sender.identity() == wanted.sender.identity())),
            "portable messaging definitions have conflicting names or sender identities"
        );
        desired.push(wanted.clone());
        let incoming_id = account_id(tenant, package_sha, &reference);
        let mut exact = None;
        for account in existing.iter().filter(|a| a.tenant_id == tenant) {
            let name_matches = normalized_name(&account.name) == name;
            let channel_matches =
                account.channel == wanted.channel && account.provider == wanted.sender.provider();
            let identity_matches =
                channel_matches && account.sender.identity() == wanted.sender.identity();
            if !(name_matches || identity_matches || account.id == incoming_id) {
                continue;
            }
            ensure!(
                name_matches && channel_matches && sender_matches(&account.sender, &wanted.sender),
                "portable messaging reference {reference} conflicts with an existing tenant account"
            );
            ensure!(
                exact.replace(account).is_none(),
                "portable messaging reference {reference} matches more than one tenant account"
            );
        }
        let binding = match exact {
            Some(account) => PortableRegistryBinding {
                account_id: account.id,
                existing_fingerprint: Some(account_fingerprint(account)?),
            },
            None => PortableRegistryBinding {
                account_id: incoming_id,
                existing_fingerprint: None,
            },
        };
        ensure!(
            bound_ids.insert(binding.account_id),
            "more than one portable messaging reference resolved to the same account"
        );
        ensure!(
            bindings.insert(reference, binding).is_none(),
            "duplicate portable messaging reference"
        );
    }
    Ok(PortableRegistryPlan {
        tenant_id: tenant,
        package_sha256: package_sha.to_owned(),
        bindings,
    })
}

async fn list_registry_accounts(
    tx: &Transaction<'_>,
    tenant: &Uuid,
) -> Result<Vec<MessagingAccount>> {
    // This projection deliberately never selects the webhook key or callback
    // URL. Account credential columns are replacement metadata only.
    tx.query(
        r#"
        SELECT id, tenant_id, channel, provider, name, sender, credentials,
               limits, provider_approval, readiness,
               (status #- '{details,callback}' #- '{details,number_changes}') AS status,
               ''::text AS webhook_key, is_default, created_at, updated_at
        FROM sequent_backend.messaging_account WHERE tenant_id = $1
        ORDER BY id
        "#,
        &[tenant],
    )
    .await?
    .into_iter()
    .map(TryInto::try_into)
    .collect()
}

pub async fn plan(
    tx: &Transaction<'_>,
    tenant: Uuid,
    package_sha: &str,
    definitions: &PortableMessagingAccounts,
) -> Result<PortableRegistryPlan> {
    let existing = list_registry_accounts(tx, &tenant).await?;
    plan_from_accounts(tenant, package_sha, definitions, &existing)
}

/// Current account views plus the new, unchecked accounts the preview plans.
/// The usual event configuration validator remains the authority on readiness.
pub async fn account_summaries(
    tx: &Transaction<'_>,
    tenant: Uuid,
    definitions: &PortableMessagingAccounts,
    expected: &PortableRegistryPlan,
) -> Result<Vec<AccountSummary>> {
    let mut existing = list_registry_accounts(tx, &tenant).await?;
    ensure!(
        plan_from_accounts(tenant, &expected.package_sha256, definitions, &existing)? == *expected,
        "portable messaging accounts changed; preview the configuration package again"
    );
    for definition in &definitions.accounts {
        let binding = &expected.bindings[&definition.reference_key()];
        if binding.existing_fingerprint.is_none() {
            let settings = settings(definition)?;
            let created_at = Utc.timestamp_opt(0, 0).single().expect("Unix epoch");
            existing.push(MessagingAccount {
                id: binding.account_id,
                tenant_id: tenant,
                channel: settings.channel,
                provider: settings.sender.provider(),
                name: settings.name,
                sender: settings.sender,
                credentials: Default::default(),
                limits: settings.limits,
                provider_approval: settings.provider_approval,
                readiness: settings.readiness,
                status: AccountCheck::default(),
                webhook_key: String::new(),
                is_default: false,
                created_at,
                updated_at: created_at,
            });
        }
    }
    Ok(existing
        .into_iter()
        .map(|mut account| {
            account.status = safe_status(&account.status);
            account_summary(&account)
        })
        .collect())
}

/// Take the registry lock before any messaging-account row locks. Checking
/// the freshly read plan under it prevents phantoms from ordinary inserts and
/// status/credential metadata changes, not only concurrent package applies.
/// The caller must use this same transaction for its ensuing registry writes.
pub async fn apply(
    tx: &Transaction<'_>,
    tenant: Uuid,
    package_sha: &str,
    definitions: &PortableMessagingAccounts,
    expected: &PortableRegistryPlan,
) -> Result<PortableRegistryPlan> {
    lock_registry(tx).await?;
    let current = plan(tx, tenant, package_sha, definitions).await?;
    ensure!(
        current == *expected,
        "portable messaging accounts changed; preview the configuration package again"
    );
    for definition in &definitions.accounts {
        let binding = &current.bindings[&definition.reference_key()];
        if binding.existing_fingerprint.is_none() {
            insert_messaging_account_with_id(
                tx,
                &tenant,
                binding.account_id,
                &settings(definition)?,
                &messaging::link::new_reference(),
            )
            .await?;
        }
    }
    let existing = list_registry_accounts(tx, &tenant).await?;
    installed_plan(tenant, package_sha, definitions, expected, &existing)
}

fn installed_plan(
    tenant: Uuid,
    package_sha: &str,
    definitions: &PortableMessagingAccounts,
    expected: &PortableRegistryPlan,
    existing: &[MessagingAccount],
) -> Result<PortableRegistryPlan> {
    const CHANGED: &str =
        "portable messaging accounts changed; preview the configuration package again";
    let current = plan_from_accounts(tenant, package_sha, definitions, existing)?;
    ensure!(
        current.tenant_id == expected.tenant_id
            && current.package_sha256 == expected.package_sha256
            && current.bindings.len() == expected.bindings.len(),
        CHANGED
    );
    for definition in &definitions.accounts {
        let key = definition.reference_key();
        let before = expected.bindings.get(&key).context(CHANGED)?;
        let now = current.bindings.get(&key).context(CHANGED)?;
        ensure!(now.account_id == before.account_id, CHANGED);
        if before.existing_fingerprint.is_some() {
            ensure!(
                now.existing_fingerprint == before.existing_fingerprint,
                CHANGED
            );
            continue;
        }
        ensure!(
            now.existing_fingerprint.is_some()
                && now.account_id == account_id(tenant, package_sha, &key),
            CHANGED
        );
        let account = existing
            .iter()
            .find(|a| a.tenant_id == tenant && a.id == now.account_id)
            .context(CHANGED)?;
        let wanted = settings(definition)?;
        ensure!(
            account.created_at == account.updated_at
                && account.name == wanted.name
                && account.sender == wanted.sender
                && account.channel == wanted.channel
                && account.provider == wanted.sender.provider()
                && account.limits == wanted.limits
                && account.provider_approval == wanted.provider_approval
                && account.readiness == wanted.readiness
                && !account.is_default
                && account.credentials.is_empty()
                && account.status == AccountCheck::default(),
            CHANGED
        );
    }
    Ok(current)
}

/// Check the installed mapping again in the native event transaction. Lock
/// only the reviewed destination accounts, not unrelated provider accounts or
/// other tenants, while the event importer provisions its realm and boards.
/// An unrelated row added after this check cannot change the pinned IDs.
pub async fn require_installed(
    tx: &Transaction<'_>,
    tenant: Uuid,
    package_sha: &str,
    definitions: &PortableMessagingAccounts,
    expected: &PortableRegistryPlan,
) -> Result<PortableRegistryPlan> {
    let ids: Vec<Uuid> = expected
        .bindings
        .values()
        .map(|binding| binding.account_id)
        .collect();
    let locked = tx
        .query(
            "SELECT id FROM sequent_backend.messaging_account
         WHERE tenant_id = $1 AND id = ANY($2) ORDER BY id FOR SHARE NOWAIT",
            &[&tenant, &ids],
        )
        .await
        .context("The destination messaging accounts cannot be locked")?;
    ensure!(
        locked.len() == ids.len(),
        "The destination messaging accounts changed since Preview"
    );
    let existing = list_registry_accounts(tx, &tenant).await?;
    installed_plan(tenant, package_sha, definitions, expected, &existing)
}

async fn lock_registry(tx: &Transaction<'_>) -> Result<()> {
    // Ordinary updates may lock a row before acquiring their table write
    // lock. EXCLUSIVE also conflicts with those existing row locks; NOWAIT
    // ensures Apply restarts instead of deadlocking with an account change.
    // Normal unlocked reads can continue.
    match tx
        .batch_execute("LOCK TABLE sequent_backend.messaging_account IN EXCLUSIVE MODE NOWAIT")
        .await
    {
        Ok(()) => Ok(()),
        Err(error)
            if error.code() == Some(&tokio_postgres::error::SqlState::LOCK_NOT_AVAILABLE) =>
        {
            Err(anyhow!(
                "portable messaging registry is busy; preview the configuration package again"
            ))
        }
        Err(error) => Err(error).context("could not lock the portable messaging registry"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::messaging::{CredentialName, CredentialRecord};

    const SHA: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    const OTHER_SHA: &str = "0000000000000000000000000000000000000000000000000000000000000001";

    fn definitions() -> PortableMessagingAccounts {
        serde_json::from_value(json!({
            "format":"sequent.tenant-messaging-accounts/1",
            "accounts":[{
                "reference":"email","name":"Demo Email",
                "sender":{"provider":"AWS_SES","from_address":"demo@example.invalid","region":"us-east-1"}
            }]
        })).unwrap()
    }

    fn with_account(id: Uuid, definitions: &PortableMessagingAccounts) -> MessagingAccount {
        let wanted = settings(&definitions.accounts[0]).unwrap();
        let now = Utc.timestamp_opt(1, 0).single().unwrap();
        MessagingAccount {
            id,
            tenant_id: Uuid::from_u128(42),
            channel: wanted.channel,
            provider: wanted.sender.provider(),
            name: wanted.name,
            sender: wanted.sender,
            credentials: Default::default(),
            limits: wanted.limits,
            provider_approval: wanted.provider_approval,
            readiness: wanted.readiness,
            status: AccountCheck::default(),
            webhook_key: "not-part-of-the-operator-preview".into(),
            is_default: false,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn preview_binds_only_its_aliases_and_new_ids_depend_on_target_digest_and_reference() {
        let tenant = Uuid::from_u128(42);
        let defs = definitions();
        let result = plan_from_accounts(tenant, SHA, &defs, &[]).unwrap();
        let alias = "package:email";
        let id = result.bindings[alias].account_id;
        assert_eq!(id, account_id(tenant, SHA, alias));
        assert_ne!(id, account_id(Uuid::from_u128(43), SHA, alias));
        assert_ne!(id, account_id(tenant, OTHER_SHA, alias));
        assert_ne!(id, account_id(tenant, SHA, "package:sms"));
        assert_eq!(result.warnings().len(), 1);
        let plain = Uuid::from_u128(200).to_string();
        let original =
            json!({"channels":[{"account_id":alias,"templates":[alias]},{"account_id":plain}]});
        let bound = result.bind(&original).unwrap();
        assert_eq!(
            bound,
            json!({"channels":[{"account_id":id,"templates":[alias]},{"account_id":plain}]})
        );
        assert!(result
            .bind(&json!({"channels":[{"account_id":"package:missing"}]}))
            .is_err());
        assert_eq!(original["channels"][0]["account_id"], alias);
    }

    #[test]
    fn existing_exact_public_identity_is_reused_but_conflict_and_ambiguity_block() {
        let tenant = Uuid::from_u128(42);
        let defs = definitions();
        let mut existing = with_account(Uuid::from_u128(19), &defs);
        existing.name = "DEMO EMAIL".into();
        existing.is_default = true;
        let reused = plan_from_accounts(tenant, SHA, &defs, &[existing.clone()]).unwrap();
        assert_eq!(reused.bindings["package:email"].account_id, existing.id);
        assert!(reused.bindings["package:email"]
            .existing_fingerprint
            .is_some());
        assert!(reused.warnings().is_empty());
        let mut other = existing.clone();
        other.id = Uuid::from_u128(20);
        assert!(plan_from_accounts(tenant, SHA, &defs, &[existing.clone(), other]).is_err());
        existing.name = "Different name with same identity".into();
        assert!(plan_from_accounts(tenant, SHA, &defs, &[existing]).is_err());
    }

    #[test]
    fn existing_meta_identity_on_a_different_api_host_conflicts() {
        let defs: PortableMessagingAccounts = serde_json::from_value(json!({
            "format":"sequent.tenant-messaging-accounts/1",
            "accounts":[{
                "reference":"messenger","name":"Demo Facebook",
                "sender":{"provider":"MESSENGER_SEND_API","page_id":"1234","api_version":"v23.0"}
            }]
        }))
        .unwrap();
        let mut record = with_account(Uuid::from_u128(12), &defs);
        if let AccountSender::MESSENGER_SEND_API { api_base_url, .. } = &mut record.sender {
            *api_base_url = Some("https://alternate.invalid".into());
        } else {
            unreachable!()
        }
        assert!(plan_from_accounts(record.tenant_id, SHA, &defs, &[record]).is_err());
    }

    #[test]
    fn new_records_must_remain_pristine_until_event_apply() {
        let tenant = Uuid::from_u128(42);
        let defs = definitions();
        let expect = plan_from_accounts(tenant, SHA, &defs, &[]).unwrap();
        let id = expect.bindings["package:email"].account_id;
        let row = with_account(id, &defs);
        assert!(installed_plan(tenant, SHA, &defs, &expect, &[]).is_err());
        assert!(installed_plan(tenant, SHA, &defs, &expect, &[row.clone()]).is_ok());
        let mut rotated = row.clone();
        rotated.credentials.insert(
            CredentialName::AWS_ACCESS_KEY_ID,
            CredentialRecord {
                replaced_at: "later".into(),
            },
        );
        let mut checked = row.clone();
        checked.status.connected = true;
        let mut edited_and_reverted = row.clone();
        edited_and_reverted.updated_at = Utc.timestamp_opt(2, 0).single().unwrap();
        let mut alternate = row.clone();
        alternate.id = Uuid::from_u128(17);
        let mut default = row;
        default.is_default = true;
        for changed in [rotated, checked, edited_and_reverted, alternate, default] {
            assert!(installed_plan(tenant, SHA, &defs, &expect, &[changed]).is_err());
        }
    }

    #[test]
    fn existing_status_and_credential_metadata_changes_invalidate_preview() {
        let tenant = Uuid::from_u128(42);
        let defs = definitions();
        let id = Uuid::from_u128(10);
        let ready = with_account(id, &defs);
        let expected = plan_from_accounts(tenant, SHA, &defs, &[ready.clone()]).unwrap();
        let serialized = serde_json::to_string(&expected).unwrap();
        assert!(!serialized.contains("not-part-of-the-operator-preview"));
        let mut rotated = ready.clone();
        rotated.credentials.insert(
            CredentialName::AWS_SECRET_ACCESS_KEY,
            CredentialRecord {
                replaced_at: "rotated".into(),
            },
        );
        assert!(installed_plan(tenant, SHA, &defs, &expected, &[rotated]).is_err());
        let mut checked = ready.clone();
        checked.status.production_access = true;
        assert!(installed_plan(tenant, SHA, &defs, &expected, &[checked]).is_err());
        assert!(installed_plan(tenant, SHA, &defs, &expected, &[ready]).is_ok());
    }

    #[test]
    fn committed_aws_credentials_are_refused_until_the_receiver_accepts_them() {
        let tenant = Uuid::from_u128(42);
        let mut defs = definitions();
        defs.accounts[0].credential_commitment = Some("ab".repeat(32));
        let refused = plan_from_accounts(tenant, SHA, &defs, &[]).unwrap_err();
        assert_eq!(refused.to_string(), CREDENTIALS_NOT_ACCEPTED);
        // Also when an existing account would otherwise be reused.
        let existing = with_account(Uuid::from_u128(19), &definitions());
        assert!(plan_from_accounts(tenant, SHA, &defs, &[existing]).is_err());
        // Omitted keeps today's behaviour: workload role or existing keys.
        assert!(plan_from_accounts(tenant, SHA, &definitions(), &[]).is_ok());
    }

    #[test]
    fn duplicate_desired_accounts_cannot_plan_two_new_rows() {
        let mut defs = definitions();
        let mut duplicate = defs.accounts[0].clone();
        duplicate.name = "Different name".into();
        duplicate.reference = "second-reference".into();
        defs.accounts.push(duplicate);
        assert!(plan_from_accounts(Uuid::from_u128(42), SHA, &defs, &[]).is_err());
    }
}
