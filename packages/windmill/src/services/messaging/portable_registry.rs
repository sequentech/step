// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Resolve signed package-local references to narrowly matched tenant accounts.
//! Planning and applying the registry never run a provider or read its secrets.

use super::accounts::check_account;
use super::config::account_summary;
use crate::postgres::messaging::{
    account_endpoint, check_account_update, empty_status, insert_messaging_account_with_id,
    update_messaging_account_reentering, AccountSettings, MessagingAccount,
};
use crate::services::import::configuration_package_credentials::{
    self as package_credentials, StagedCredentials,
};
use anyhow::{anyhow, ensure, Context, Result};
use chrono::{TimeZone, Utc};
use deadpool_postgres::Transaction;
use sequent_core::election_config::portable_messaging::{
    DeliveredCredentialKind, PortableMessagingAccount, PortableMessagingAccounts, PortableSender,
    REF_PREFIX,
};
use sequent_core::types::messaging::{
    AccountCheck, AccountLimits, AccountSender, AccountSummary, CredentialName, ProviderApproval,
    ReadinessPolicy,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RegistryAction {
    Create,
    Reuse,
    Replace,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableRegistryBinding {
    pub account_id: Uuid,
    pub action: RegistryAction,
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

    pub fn needs_apply(&self) -> bool {
        self.bindings
            .values()
            .any(|binding| binding.action != RegistryAction::Reuse)
    }
}

/// The only vault credential names a delivery may install: an explicitly
/// signed AWS pair, or an explicitly signed Infobip API key. An omitted
/// commitment never deletes or changes local credentials.
pub(crate) fn installed_credential_names(
    definition: &PortableMessagingAccount,
) -> BTreeSet<CredentialName> {
    if definition.credential_commitment.is_none() {
        return BTreeSet::new();
    }
    match definition.sender.delivered_credential() {
        Some(DeliveredCredentialKind::AWS_KEY_PAIR) => BTreeSet::from([
            CredentialName::AWS_ACCESS_KEY_ID,
            CredentialName::AWS_SECRET_ACCESS_KEY,
        ]),
        Some(DeliveredCredentialKind::API_KEY) => BTreeSet::from([CredentialName::API_KEY]),
        None => BTreeSet::new(),
    }
}

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

#[derive(Debug, Serialize)]
struct ManagedAccount {
    account_id: Uuid,
    applied_package_sha256: Option<String>,
    credential_metadata: Option<Value>,
}

type Ownership = BTreeMap<String, ManagedAccount>;

fn account_fingerprint(
    account: &MessagingAccount,
    owner: Option<&ManagedAccount>,
) -> Result<String> {
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
        "portable_binding": owner,
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

/// Merge only fields the portable format owns. A package cannot set the
/// local callback endpoint, notification topic, template approval, limits,
/// default selection or other tenant policy.
fn replacement_settings(
    definition: &PortableMessagingAccount,
    current: &MessagingAccount,
) -> Result<AccountSettings> {
    let reference = definition.reference_key();
    let mut next = settings(definition)?;
    ensure!(
        current.channel == next.channel && current.provider == next.sender.provider(),
        "portable messaging reference {reference} is bound to a different provider or channel; choose a different reference"
    );
    let same_identity = current.sender.identity() == next.sender.identity();
    match (&current.sender, &mut next.sender) {
        (
            AccountSender::AWS_SES {
                notification_topic_arn: saved,
                ..
            },
            AccountSender::AWS_SES {
                notification_topic_arn,
                ..
            },
        ) => {
            ensure!(
                same_identity || saved.is_none(),
                "portable messaging reference {reference} cannot move a sender with a tenant-managed notification topic"
            );
            notification_topic_arn.clone_from(saved);
        }
        (
            AccountSender::WHATSAPP_CLOUD_API {
                api_base_url: saved,
                ..
            },
            AccountSender::WHATSAPP_CLOUD_API { api_base_url, .. },
        )
        | (
            AccountSender::MESSENGER_SEND_API {
                api_base_url: saved,
                ..
            },
            AccountSender::MESSENGER_SEND_API { api_base_url, .. },
        ) => {
            ensure!(
                same_identity || saved.is_none(),
                "portable messaging reference {reference} cannot move a sender with a tenant-managed API endpoint"
            );
            api_base_url.clone_from(saved);
        }
        (
            AccountSender::VIBER_INFOBIP {
                approved_templates: saved,
                content_mode: saved_mode,
                ..
            },
            AccountSender::VIBER_INFOBIP {
                approved_templates,
                content_mode,
                ..
            },
        ) => {
            // What the package declares is the package's; what it leaves
            // out stays as the tenant set it.
            let PortableSender::VIBER_INFOBIP {
                content_mode: declared_mode,
                approved_templates: declared,
                ..
            } = &definition.sender
            else {
                unreachable!("a Viber account is declared as Viber");
            };
            if declared.is_none() {
                ensure!(
                    same_identity || saved.is_empty(),
                    "portable messaging reference {reference} cannot move a sender with tenant-managed template approvals"
                );
                approved_templates.clone_from(saved);
            }
            if declared_mode.is_none() {
                *content_mode = *saved_mode;
            }
        }
        _ => {}
    }
    next.limits.clone_from(&current.limits);
    next.is_default = current.is_default;
    next.provider_approval = if same_identity {
        current.provider_approval
    } else {
        ProviderApproval::PENDING
    };
    check_account_update(current, &next, &installed_credential_names(definition)).map_err(|_| {
        anyhow!("portable messaging reference {reference} requires a separate tenant account change before importing")
    })?;
    Ok(next)
}

fn has_pair(account: &MessagingAccount, definition: &PortableMessagingAccount) -> bool {
    installed_credential_names(definition)
        .iter()
        .all(|name| account.credentials.contains_key(name))
}

fn keys_already_installed(
    definition: &PortableMessagingAccount,
    account: &MessagingAccount,
    owner: &ManagedAccount,
    package_sha: &str,
) -> Result<bool> {
    Ok(definition.credential_commitment.is_none()
        || (owner.applied_package_sha256.as_deref() == Some(package_sha)
            && has_pair(account, definition)
            && owner.credential_metadata.as_ref()
                == Some(&serde_json::to_value(&account.credentials)?)))
}

fn plan_from_registry(
    tenant: Uuid,
    package_sha: &str,
    definitions: &PortableMessagingAccounts,
    existing: &[MessagingAccount],
    ownership: &Ownership,
) -> Result<PortableRegistryPlan> {
    package_digest(package_sha)?;
    definitions.validate()?;
    let mut bindings = BTreeMap::new();
    let mut bound_ids = BTreeSet::new();
    for definition in &definitions.accounts {
        let reference = definition.reference_key();
        let wanted = settings(definition)?;
        let name = normalized_name(&wanted.name);
        let incoming_id = account_id(tenant, package_sha, &reference);
        let owner = ownership.get(&reference);
        let mut exact = None;
        for account in existing.iter().filter(|a| a.tenant_id == tenant) {
            if owner.is_some_and(|owner| account.id == owner.account_id) {
                ensure!(
                    exact.replace(account).is_none(),
                    "portable messaging reference {reference} matches more than one tenant account"
                );
                continue;
            }
            let name_matches = normalized_name(&account.name) == name;
            let provider_matches =
                account.channel == wanted.channel && account.provider == wanted.sender.provider();
            let identity_matches =
                provider_matches && account.sender.identity() == wanted.sender.identity();
            if !(name_matches || identity_matches || account.id == incoming_id) {
                continue;
            }
            // A second candidate is always ambiguous. A registered alias
            // also cannot be claimed through a different package reference.
            ensure!(
                owner.is_none()
                    && !ownership.values().any(|owner| owner.account_id == account.id)
                    && name_matches
                    && provider_matches
                    && sender_matches(&account.sender, &wanted.sender),
                "portable messaging reference {reference} conflicts with an existing tenant account not bound to that reference"
            );
            ensure!(
                exact.replace(account).is_none(),
                "portable messaging reference {reference} matches more than one tenant account"
            );
        }
        let (id, action, fingerprint) = match (owner, exact) {
            (Some(owner), Some(account)) => {
                let next = replacement_settings(definition, account)?;
                let same = account.name == next.name && account.sender == next.sender;
                // A legacy receiver derived its ID from these exact bytes.
                // Backfilled rows have no prior applied-package receipt.
                let legacy_retry = owner.applied_package_sha256.is_none()
                    && account.id == incoming_id
                    && account.created_at == account.updated_at
                    && has_pair(account, definition);
                let action = if same
                    && (legacy_retry
                        || keys_already_installed(definition, account, owner, package_sha)?)
                {
                    RegistryAction::Reuse
                } else {
                    RegistryAction::Replace
                };
                (
                    account.id,
                    action,
                    Some(account_fingerprint(account, Some(owner))?),
                )
            }
            (None, Some(account)) => {
                // Do not adopt a locally managed account because it looks
                // like this sender. Preserve exact, credential-free reuse,
                // and keep support for finishing a legacy same-byte install.
                ensure!(
                    definition.credential_commitment.is_none()
                        || (account.id == incoming_id && account.created_at == account.updated_at
                            && has_pair(account, definition)),
                    "portable messaging reference {reference} already exists on this tenant; its credentials are managed here, not by a package"
                );
                (
                    account.id,
                    RegistryAction::Reuse,
                    Some(account_fingerprint(account, None)?),
                )
            }
            (None, None) => (incoming_id, RegistryAction::Create, None),
            (Some(_), None) => {
                return Err(anyhow!(
                    "portable messaging reference {reference} has no matching tenant account"
                ));
            }
        };
        ensure!(
            bound_ids.insert(id),
            "more than one portable messaging reference resolved to the same account"
        );
        bindings.insert(
            reference,
            PortableRegistryBinding {
                account_id: id,
                action,
                existing_fingerprint: fingerprint,
            },
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

async fn list_ownership(tx: &Transaction<'_>, tenant: &Uuid) -> Result<Ownership> {
    tx.query(
        "SELECT reference, account_id, applied_package_sha256, credential_metadata
         FROM sequent_backend.portable_messaging_account_binding WHERE tenant_id = $1",
        &[tenant],
    )
    .await?
    .into_iter()
    .map(|row| {
        Ok((
            row.try_get(0)?,
            ManagedAccount {
                account_id: row.try_get(1)?,
                applied_package_sha256: row.try_get(2)?,
                credential_metadata: row.try_get(3)?,
            },
        ))
    })
    .collect()
}

const CHANGED: &str =
    "portable messaging accounts changed; preview the configuration package again";

pub async fn plan(
    tx: &Transaction<'_>,
    tenant: Uuid,
    package_sha: &str,
    definitions: &PortableMessagingAccounts,
) -> Result<PortableRegistryPlan> {
    let existing = list_registry_accounts(tx, &tenant).await?;
    let ownership = list_ownership(tx, &tenant).await?;
    plan_from_registry(tenant, package_sha, definitions, &existing, &ownership)
}

fn fresh_account(tenant: Uuid, id: Uuid, wanted: AccountSettings) -> MessagingAccount {
    let now = Utc.timestamp_opt(0, 0).single().expect("Unix epoch");
    MessagingAccount {
        id,
        tenant_id: tenant,
        channel: wanted.channel,
        provider: wanted.sender.provider(),
        name: wanted.name,
        sender: wanted.sender,
        credentials: Default::default(),
        limits: wanted.limits,
        provider_approval: wanted.provider_approval,
        readiness: wanted.readiness,
        status: AccountCheck::default(),
        webhook_key: String::new(),
        is_default: false,
        created_at: now,
        updated_at: now,
    }
}

/// Project exactly what the verified package would install, without writing
/// or borrowing a displaced account's successful provider check.
pub async fn account_summaries(
    tx: &Transaction<'_>,
    tenant: Uuid,
    definitions: &PortableMessagingAccounts,
    expected: &PortableRegistryPlan,
) -> Result<Vec<AccountSummary>> {
    let mut existing = list_registry_accounts(tx, &tenant).await?;
    let ownership = list_ownership(tx, &tenant).await?;
    ensure!(
        plan_from_registry(
            tenant,
            &expected.package_sha256,
            definitions,
            &existing,
            &ownership
        )? == *expected,
        CHANGED
    );
    for definition in &definitions.accounts {
        let binding = &expected.bindings[&definition.reference_key()];
        match binding.action {
            RegistryAction::Reuse => {}
            RegistryAction::Create => {
                existing.push(fresh_account(
                    tenant,
                    binding.account_id,
                    settings(definition)?,
                ));
            }
            RegistryAction::Replace => {
                let account = existing
                    .iter_mut()
                    .find(|a| a.id == binding.account_id)
                    .context(CHANGED)?;
                let wanted = replacement_settings(definition, account)?;
                account.name = wanted.name;
                account.sender = wanted.sender;
                account.provider_approval = wanted.provider_approval;
                account.readiness = wanted.readiness;
                account.status = AccountCheck::default();
            }
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

async fn remember_install(
    tx: &Transaction<'_>,
    tenant: &Uuid,
    id: &Uuid,
    reference: &str,
    digest: &str,
) -> Result<()> {
    let changed = tx
        .execute(
            "INSERT INTO sequent_backend.portable_messaging_account_binding AS binding
             (tenant_id, reference, account_id, applied_package_sha256, credential_metadata)
         SELECT tenant_id, $3, id, $4, credentials FROM sequent_backend.messaging_account
         WHERE tenant_id = $1 AND id = $2
         ON CONFLICT (tenant_id, reference) DO UPDATE SET
             applied_package_sha256 = EXCLUDED.applied_package_sha256,
             credential_metadata = EXCLUDED.credential_metadata
         WHERE binding.account_id = EXCLUDED.account_id",
            &[tenant, id, &reference, &digest],
        )
        .await?;
    ensure!(changed == 1, CHANGED);
    Ok(())
}

/// Hold the registry lock before any account row locks. No settings, vault,
/// or provenance write happens until the signed Preview plan is rechecked.
pub async fn apply(
    tx: &Transaction<'_>,
    tenant: Uuid,
    package_sha: &str,
    definitions: &PortableMessagingAccounts,
    expected: &PortableRegistryPlan,
    credentials: Option<&StagedCredentials>,
) -> Result<PortableRegistryPlan> {
    lock_registry(tx).await?;
    let existing = list_registry_accounts(tx, &tenant).await?;
    let owners = list_ownership(tx, &tenant).await?;
    let current = plan_from_registry(tenant, package_sha, definitions, &existing, &owners)?;
    ensure!(current == *expected, CHANGED);
    // Resolve all required verified input before the first write.
    let mut delivered = BTreeMap::new();
    for definition in &definitions.accounts {
        let reference = definition.reference_key();
        if definition.credential_commitment.is_some()
            && current.bindings[&reference].action != RegistryAction::Reuse
        {
            let credential = credentials
                .and_then(|staged| staged.get(&reference))
                .ok_or_else(|| anyhow!(package_credentials::NOT_ACCEPTED))?;
            delivered.insert(reference, credential);
        }
    }
    for definition in &definitions.accounts {
        let reference = definition.reference_key();
        let binding = &current.bindings[&reference];
        let id = &binding.account_id;
        match binding.action {
            RegistryAction::Reuse => continue,
            RegistryAction::Create => {
                insert_messaging_account_with_id(
                    tx,
                    &tenant,
                    *id,
                    &settings(definition)?,
                    &messaging::link::new_reference(),
                )
                .await?;
            }
            RegistryAction::Replace => {
                let account = existing.iter().find(|a| a.id == *id).context(CHANGED)?;
                update_messaging_account_reentering(
                    tx,
                    &tenant,
                    id,
                    &replacement_settings(definition, account)?,
                    &installed_credential_names(definition),
                )
                .await?
                .context(CHANGED)?;
                // The prior sender's approval/check never becomes evidence
                // for the replacement. Keep safety history, not readiness.
                let changed = tx
                    .execute(
                        "UPDATE sequent_backend.messaging_account
                     SET status = jsonb_set($3::jsonb, '{details,number_changes}',
                             COALESCE(status#>'{details,number_changes}', '[]'::jsonb)),
                         readiness = 'PROVIDER_CHECK', updated_at = now()
                     WHERE tenant_id = $1 AND id = $2",
                        &[&tenant, id, &empty_status()?],
                    )
                    .await?;
                ensure!(changed == 1, CHANGED);
            }
        }
        if let Some(credential) = delivered.get(&reference) {
            package_credentials::install(tx, &tenant, id, credential).await?;
        }
        remember_install(tx, &tenant, id, &reference, package_sha).await?;
    }
    let applied = list_registry_accounts(tx, &tenant).await?;
    let owners = list_ownership(tx, &tenant).await?;
    let installed = plan_from_registry(tenant, package_sha, definitions, &applied, &owners)?;
    ensure!(!installed.needs_apply(), CHANGED);
    for definition in &definitions.accounts {
        let reference = definition.reference_key();
        let before = &current.bindings[&reference];
        let after = &installed.bindings[&reference];
        ensure!(before.account_id == after.account_id, CHANGED);
        if before.action == RegistryAction::Reuse {
            ensure!(after == before, CHANGED);
            continue;
        }
        let row = applied
            .iter()
            .find(|a| a.id == before.account_id)
            .context(CHANGED)?;
        let prior = existing.iter().find(|a| a.id == before.account_id);
        let wanted = match prior {
            Some(prior) => replacement_settings(definition, prior)?,
            None => settings(definition)?,
        };
        ensure!(
            row.name == wanted.name
                && row.sender == wanted.sender
                && row.provider_approval == wanted.provider_approval
                && row.readiness == wanted.readiness
                && row.is_default == wanted.is_default
                && row.limits == wanted.limits
                && row.status == AccountCheck::default()
                && prior.is_none_or(|prior| row.created_at == prior.created_at)
                && (definition.credential_commitment.is_some()
                    || prior.map_or_else(
                        || row.credentials.is_empty(),
                        |prior| row.credentials == prior.credentials
                    )),
            CHANGED
        );
    }
    Ok(installed)
}

/// Check, with their providers, the accounts this Apply created or replaced,
/// in the transaction that wrote them (so their rows are still locked), and
/// read the installed plan again. A check only stores the account's status,
/// which the installed plan's fingerprints include: the event import, which
/// compares them, then sees each account as it was checked here, and the
/// purposes a passing check makes ready stay on. A provider that can't be
/// reached, or an account without its credentials, stores why instead; that
/// never fails the import, and those purposes stay off as before.
pub async fn check_written(
    tx: &Transaction<'_>,
    tenant: Uuid,
    package_sha: &str,
    definitions: &PortableMessagingAccounts,
    planned: &PortableRegistryPlan,
    endpoints: &messaging::providers::Endpoints,
) -> Result<PortableRegistryPlan> {
    for binding in planned.bindings.values() {
        if binding.action == RegistryAction::Reuse {
            continue;
        }
        // A database error aborts the transaction, and the reads below
        // report it; anything else (an unreadable credential, say) leaves
        // the account unchecked, as the daily check would.
        if let Err(error) = check_account(tx, &tenant, &binding.account_id, endpoints).await {
            tracing::warn!(account = %binding.account_id, "package account check could not run: {error:#}");
        }
    }
    let applied = list_registry_accounts(tx, &tenant).await?;
    let owners = list_ownership(tx, &tenant).await?;
    let installed = plan_from_registry(tenant, package_sha, definitions, &applied, &owners)?;
    ensure!(!installed.needs_apply(), CHANGED);
    ensure!(
        installed.bindings.len() == planned.bindings.len()
            && installed.bindings.iter().all(|(reference, binding)| {
                planned
                    .bindings
                    .get(reference)
                    .is_some_and(|before| before.account_id == binding.account_id)
            }),
        CHANGED
    );
    Ok(installed)
}

/// Recheck the exact applied plan in the native event transaction. A change
/// to a reviewed account, including its credentials or provenance, cannot
/// silently retarget or inherit the receipt of a completed Write.
pub async fn require_installed(
    tx: &Transaction<'_>,
    tenant: Uuid,
    package_sha: &str,
    definitions: &PortableMessagingAccounts,
    expected: &PortableRegistryPlan,
) -> Result<PortableRegistryPlan> {
    ensure!(!expected.needs_apply(), CHANGED);
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
    ensure!(locked.len() == ids.len(), CHANGED);
    let current = plan(tx, tenant, package_sha, definitions).await?;
    ensure!(current == *expected, CHANGED);
    Ok(current)
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

    fn plan_from_accounts(
        tenant: Uuid,
        digest: &str,
        definitions: &PortableMessagingAccounts,
        existing: &[MessagingAccount],
    ) -> Result<PortableRegistryPlan> {
        plan_from_registry(tenant, digest, definitions, existing, &Ownership::new())
    }

    fn owner(row: &MessagingAccount, digest: Option<&str>) -> Ownership {
        BTreeMap::from([(
            "package:email".into(),
            ManagedAccount {
                account_id: row.id,
                applied_package_sha256: digest.map(str::to_owned),
                credential_metadata: digest
                    .map(|_| serde_json::to_value(&row.credentials).unwrap()),
            },
        )])
    }

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
        assert!(result.needs_apply());
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
        assert!(!reused.needs_apply());
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
    fn only_a_tracked_same_tenant_reference_can_plan_a_replacement() {
        let tenant = Uuid::from_u128(42);
        let mut defs = definitions();
        let row = with_account(account_id(tenant, SHA, "package:email"), &defs);
        let mut next = serde_json::to_value(&defs).unwrap();
        next["accounts"][0]["sender"]["region"] = json!("us-east-2");
        defs = serde_json::from_value(next).unwrap();
        assert!(plan_from_accounts(tenant, OTHER_SHA, &defs, &[row.clone()]).is_err());
        let owners = owner(&row, Some(SHA));
        let replace =
            plan_from_registry(tenant, OTHER_SHA, &defs, &[row.clone()], &owners).unwrap();
        assert!(replace.needs_apply());
        assert_eq!(
            replace.bindings["package:email"].action,
            RegistryAction::Replace
        );
        assert_eq!(replace.bindings["package:email"].account_id, row.id);

        let other_tenant = Uuid::from_u128(43);
        let elsewhere = plan_from_accounts(other_tenant, OTHER_SHA, &defs, &[row.clone()]).unwrap();
        assert_eq!(
            elsewhere.bindings["package:email"].action,
            RegistryAction::Create
        );
        assert_ne!(elsewhere.bindings["package:email"].account_id, row.id);

        let mut wrong_name = defs.clone();
        wrong_name.accounts[0].reference = "another-alias".into();
        assert!(
            plan_from_registry(tenant, OTHER_SHA, &wrong_name, &[row.clone()], &owners).is_err()
        );
        let mut colliding = with_account(Uuid::from_u128(22), &defs);
        colliding.name = "Verified manual alias".into();
        assert!(
            plan_from_registry(tenant, OTHER_SHA, &defs, &[row.clone(), colliding], &owners)
                .is_err()
        );
        let mut next = serde_json::to_value(&defs).unwrap();
        next["accounts"][0]["sender"] = json!({"provider":"AWS_SNS","sender_id":"DEMO"});
        let different_provider = serde_json::from_value(next).unwrap();
        assert!(
            plan_from_registry(tenant, OTHER_SHA, &different_provider, &[row], &owners)
                .unwrap_err()
                .to_string()
                .contains("different provider or channel")
        );
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
        assert_ne!(
            expected,
            plan_from_accounts(tenant, SHA, &defs, &[rotated]).unwrap()
        );
        let mut checked = ready.clone();
        checked.status.production_access = true;
        assert_ne!(
            expected,
            plan_from_accounts(tenant, SHA, &defs, &[checked]).unwrap()
        );
        assert_eq!(
            expected,
            plan_from_accounts(tenant, SHA, &defs, &[ready]).unwrap()
        );
    }

    #[test]
    fn committed_keys_go_only_to_the_account_this_package_creates() {
        let tenant = Uuid::from_u128(42);
        let mut defs = definitions();
        defs.accounts[0].credential_commitment = Some("ab".repeat(32));
        let created = plan_from_accounts(tenant, SHA, &defs, &[]).unwrap();
        let id = created.bindings["package:email"].account_id;
        assert!(created.bindings["package:email"]
            .existing_fingerprint
            .is_none());
        // An installation's own account is never overwritten by a package.
        let existing = with_account(Uuid::from_u128(19), &definitions());
        let refused = plan_from_accounts(tenant, SHA, &defs, &[existing])
            .unwrap_err()
            .to_string();
        assert!(refused.starts_with("portable messaging reference package:email already exists"));
        assert!(!refused.contains(&"ab".repeat(32)));
        // Resuming: the row this package created (same derived ID) is reused.
        let mut resumed = with_account(id, &defs);
        resumed.credentials = BTreeMap::from([
            (
                CredentialName::AWS_ACCESS_KEY_ID,
                CredentialRecord {
                    replaced_at: "t".into(),
                },
            ),
            (
                CredentialName::AWS_SECRET_ACCESS_KEY,
                CredentialRecord {
                    replaced_at: "t".into(),
                },
            ),
        ]);
        assert!(plan_from_accounts(tenant, SHA, &defs, &[resumed]).is_ok());
        // Omitted keeps today's behaviour: workload role or existing keys.
        let existing = with_account(Uuid::from_u128(19), &definitions());
        assert!(plan_from_accounts(tenant, SHA, &definitions(), &[existing]).is_ok());
    }

    #[test]
    fn only_the_exact_installed_pair_receipt_is_idempotent() {
        let tenant = Uuid::from_u128(42);
        let mut defs = definitions();
        defs.accounts[0].credential_commitment = Some("ab".repeat(32));
        let id = account_id(tenant, SHA, "package:email");
        let mut row = with_account(id, &defs);
        for key in installed_credential_names(&defs.accounts[0]) {
            row.credentials.insert(
                key,
                CredentialRecord {
                    replaced_at: "before".into(),
                },
            );
        }
        let owners = owner(&row, Some(SHA));
        assert!(
            !plan_from_registry(tenant, SHA, &defs, &[row.clone()], &owners)
                .unwrap()
                .needs_apply()
        );
        assert_eq!(
            plan_from_registry(tenant, OTHER_SHA, &defs, &[row.clone()], &owners)
                .unwrap()
                .bindings["package:email"]
                .action,
            RegistryAction::Replace
        );
        row.credentials
            .get_mut(&CredentialName::AWS_SECRET_ACCESS_KEY)
            .unwrap()
            .replaced_at = "manual rotation".into();
        assert_eq!(
            plan_from_registry(tenant, SHA, &defs, &[row], &owners)
                .unwrap()
                .bindings["package:email"]
                .action,
            RegistryAction::Replace
        );
    }

    fn infobip(sender: Value) -> PortableMessagingAccounts {
        serde_json::from_value(json!({
            "format":"sequent.tenant-messaging-accounts/1",
            "accounts":[{"reference":"infobip","name":"Demo Infobip","sender":sender}]
        }))
        .unwrap()
    }

    fn viber(extra: Value) -> PortableMessagingAccounts {
        let mut sender = json!({"provider":"VIBER_INFOBIP",
            "base_url":"https://abc-1.api.infobip.com","sender":"Demo"});
        for (k, v) in extra.as_object().unwrap() {
            sender[k] = v.clone();
        }
        infobip(sender)
    }

    #[test]
    fn each_committed_account_installs_only_its_own_kind_of_credential() {
        let whatsapp = infobip(json!({"provider":"WHATSAPP_INFOBIP",
            "base_url":"https://abc-1.api.infobip.com","sender_number":"447796344125"}));
        let mut committed = whatsapp.clone();
        committed.accounts[0].credential_commitment = Some("ab".repeat(32));
        assert_eq!(
            installed_credential_names(&committed.accounts[0]),
            BTreeSet::from([CredentialName::API_KEY])
        );
        let mut viber = viber(json!({}));
        viber.accounts[0].credential_commitment = Some("ab".repeat(32));
        assert_eq!(
            installed_credential_names(&viber.accounts[0]),
            BTreeSet::from([CredentialName::API_KEY])
        );
        let mut aws = definitions();
        aws.accounts[0].credential_commitment = Some("ab".repeat(32));
        assert_eq!(
            installed_credential_names(&aws.accounts[0]),
            BTreeSet::from([
                CredentialName::AWS_ACCESS_KEY_ID,
                CredentialName::AWS_SECRET_ACCESS_KEY
            ])
        );
        // Without a commitment nothing is installed, and nothing removed.
        assert!(installed_credential_names(&whatsapp.accounts[0]).is_empty());
    }

    #[test]
    fn whatsapp_through_infobip_is_created_from_its_declaration() {
        let tenant = Uuid::from_u128(42);
        let defs = infobip(json!({"provider":"WHATSAPP_INFOBIP",
            "base_url":"https://abc-1.api.infobip.com","sender_number":"447796344125",
            "display_name":"Demo Elections"}));
        let wanted = settings(&defs.accounts[0]).unwrap();
        assert_eq!(
            wanted.channel,
            sequent_core::types::messaging::MessageChannel::WHATSAPP
        );
        assert_eq!(
            wanted.sender.provider(),
            sequent_core::types::messaging::MessagingProvider::WHATSAPP_INFOBIP
        );
        let plan = plan_from_accounts(tenant, SHA, &defs, &[]).unwrap();
        assert_eq!(
            plan.bindings["package:infobip"].action,
            RegistryAction::Create
        );
        // Without a key, an existing account of the same number is reused.
        let existing = with_account(Uuid::from_u128(30), &defs);
        let reused = plan_from_accounts(tenant, SHA, &defs, &[existing.clone()]).unwrap();
        assert_eq!(
            reused.bindings["package:infobip"].action,
            RegistryAction::Reuse
        );
        assert_eq!(reused.bindings["package:infobip"].account_id, existing.id);
    }

    #[test]
    fn viber_keeps_its_own_mode_and_approvals_unless_the_package_declares_them() {
        use sequent_core::types::messaging::{MessagePurpose, ViberContentMode};
        let tenant = Uuid::from_u128(42);
        let plain = viber(json!({}));
        let mut row = with_account(Uuid::from_u128(31), &plain);
        let entered = BTreeMap::from([(
            MessagePurpose::OTP,
            BTreeMap::from([("en".to_string(), "entered-here".to_string())]),
        )]);
        if let AccountSender::VIBER_INFOBIP {
            content_mode,
            approved_templates,
            ..
        } = &mut row.sender
        {
            *content_mode = ViberContentMode::FREE_TEXT;
            *approved_templates = entered.clone();
        }
        let owners: Ownership = BTreeMap::from([(
            "package:infobip".into(),
            ManagedAccount {
                account_id: row.id,
                applied_package_sha256: Some(SHA.into()),
                credential_metadata: Some(json!({})),
            },
        )]);
        let sender_of = |defs: &PortableMessagingAccounts| {
            replacement_settings(&defs.accounts[0], &row)
                .unwrap()
                .sender
        };
        // Silent: the receiver's mode and approvals stay, so nothing changes.
        assert_eq!(sender_of(&plain), row.sender);
        assert_eq!(
            plan_from_registry(tenant, OTHER_SHA, &plain, &[row.clone()], &owners)
                .unwrap()
                .bindings["package:infobip"]
                .action,
            RegistryAction::Reuse
        );
        // Declared: the package's own mode and approvals replace them.
        let declared = viber(json!({"content_mode":"APPROVED_TEMPLATES",
            "approved_templates":{"OTP":{"en":"from-package"},"NOTICE":{"en":"notice"}}}));
        let AccountSender::VIBER_INFOBIP {
            content_mode,
            approved_templates,
            ..
        } = sender_of(&declared)
        else {
            panic!("VIBER_INFOBIP");
        };
        assert_eq!(content_mode, ViberContentMode::APPROVED_TEMPLATES);
        assert_eq!(
            approved_templates[&MessagePurpose::OTP]["en"],
            "from-package"
        );
        assert_eq!(
            plan_from_registry(tenant, OTHER_SHA, &declared, &[row.clone()], &owners)
                .unwrap()
                .bindings["package:infobip"]
                .action,
            RegistryAction::Replace
        );
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
