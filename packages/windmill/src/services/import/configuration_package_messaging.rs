// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Receiver-owned planning for a signed package's public sending accounts.
//! The signed bytes are never rewritten. The import resolves them in its own
//! transaction to exactly the target accounts the operator previewed.

use crate::services::import::configuration_package_apply::SectionPreview;
use crate::services::import::configuration_package_credentials as package_credentials;
use crate::services::messaging::config::{with_original_account_ids, ProviderReadiness};
use crate::services::messaging::portable_registry::{
    self, DeclaredChecks, PortableRegistryPlan, RegistryAction,
};
use anyhow::{anyhow, ensure, Result};
use deadpool_postgres::Transaction;
use sequent_core::election_config::portable_messaging::{
    validate_event_references, DeclaredLanguages, DeliveredCredentialKind,
    PortableMessagingAccounts, REF_PREFIX,
};
use sequent_core::signing::sha256_hex;
use sequent_core::types::messaging::{
    AccountSummary, EventMessagingConfig, MessageChannel, MessagePurpose, MessagingConfigError,
    ReadinessBlocker,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::OnceLock;
use uuid::Uuid;

pub(crate) const STALE: &str =
    "The destination messaging accounts changed since Preview. Preview this signed package again before applying.";

pub struct PackageMessaging {
    tenant: Uuid,
    digest: String,
    definitions: PortableMessagingAccounts,
    confirmed: Option<String>,
    /// The protected staging document whose vault entry holds any
    /// credentials (AWS key pairs, Infobip API keys) delivered with this
    /// package. Set by the server, never the package.
    staged_document: Option<Uuid>,
    /// The task's verified actor (the JWT-derived audit user ID Harvest put
    /// in the task body): staged credentials are only for their uploader.
    actor: Option<String>,
    /// What each account the package creates or replaces is taken to be:
    /// checked, as its author checked it. Apply never asks a provider.
    declared: DeclaredChecks,
    snapshot: OnceLock<PortableRegistryPlan>,
    installed_here: OnceLock<PortableRegistryPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PendingMessaging {
    pub channel: MessageChannel,
    pub purposes: BTreeSet<MessagePurpose>,
}

pub(crate) struct PreparedMessaging {
    /// The event's settings as signed, bound to this tenant's accounts.
    pub config: Value,
    /// What isn't ready yet on the package's accounts: kept, and said.
    pub pending: Vec<PendingMessaging>,
    /// The readiness the settings are stored with: the package's own
    /// channels may still wait for their accounts.
    pub readiness: ProviderReadiness,
}

/// The template languages the package names for its accounts
/// ([`PortableMessagingAccounts::declared_languages`]), read from the signed
/// event's messaging settings in `event_zip` and the provider languages of
/// the package's message templates (`notice_templates`). Settings that don't
/// read name none here: the event import refuses them itself.
pub(crate) fn declared_languages(
    definitions: &PortableMessagingAccounts,
    event_zip: Option<&[u8]>,
    notice_templates: &[(MessageChannel, String)],
) -> DeclaredLanguages {
    let event = event_zip
        .and_then(|zip| super::configuration_package_sections::event_document(zip).ok())
        .and_then(|document| serde_json::from_slice::<Value>(&document).ok())
        .and_then(|document| document.get("messaging_config").cloned())
        .and_then(|config| serde_json::from_value::<EventMessagingConfig>(config).ok());
    definitions.declared_languages(event.as_ref(), notice_templates)
}

impl PackageMessaging {
    pub(crate) fn new(
        tenant: Uuid,
        digest: String,
        definitions: PortableMessagingAccounts,
        confirmed: Option<String>,
        staged_document: Option<Uuid>,
        actor: Option<String>,
        languages: &DeclaredLanguages,
    ) -> Self {
        let checked_at = chrono::Utc::now().to_rfc3339();
        let declared = definitions
            .accounts
            .iter()
            .map(|definition| {
                (
                    definition.reference_key(),
                    definition.declared_check(languages, checked_at.clone()),
                )
            })
            .collect();
        Self {
            tenant,
            digest,
            definitions,
            confirmed,
            staged_document,
            actor,
            declared,
            snapshot: OnceLock::new(),
            installed_here: OnceLock::new(),
        }
    }

    /// A create or replacement which commits keys requires its verified
    /// sidecar. Reusing an exactly installed pair does not.
    fn needs_credentials(&self, plan: &PortableRegistryPlan) -> bool {
        self.definitions.accounts.iter().any(|definition| {
            definition.credential_commitment.is_some()
                && plan
                    .bindings
                    .get(&definition.reference_key())
                    .is_some_and(|binding| binding.action != RegistryAction::Reuse)
        })
    }

    /// Always consulted, so a staged sidecar is checked (uploader, package,
    /// expiry, commitments); it is consumed atomically whenever this
    /// Apply writes the section. A resumed read-only match needs none.
    async fn credentials(
        &self,
        tx: &Transaction<'_>,
        plan: &PortableRegistryPlan,
    ) -> Result<Option<package_credentials::StagedCredentials>> {
        package_credentials::load(
            tx,
            &self.tenant,
            &self.digest,
            self.staged_document.as_ref(),
            &self.definitions,
            self.needs_credentials(plan),
            self.actor.as_deref(),
        )
        .await
    }

    pub(crate) fn fingerprint(&self) -> Option<String> {
        self.snapshot.get().map(|plan| {
            serde_json::to_vec(plan)
                .map(|bytes| sha256_hex(&bytes))
                .expect("a registry plan can be encoded")
        })
    }

    pub(crate) async fn plan(&self, tx: &Transaction<'_>) -> Result<PortableRegistryPlan> {
        if let Some(installed) = self.installed_here.get() {
            return portable_registry::require_installed(
                tx,
                self.tenant,
                &self.digest,
                &self.definitions,
                installed,
            )
            .await;
        }
        let plan =
            portable_registry::plan(tx, self.tenant, &self.digest, &self.definitions).await?;
        if let Some(expected) = &self.confirmed {
            ensure!(
                sha256_hex(&serde_json::to_vec(&plan)?) == *expected,
                "{STALE}"
            );
        }
        let pinned = self.snapshot.get_or_init(|| plan.clone());
        ensure!(pinned == &plan, "{STALE}");
        // Preview refuses a missing or mismatched sidecar now, not at Apply.
        self.credentials(tx, &plan).await?;
        Ok(plan)
    }

    pub(crate) async fn preview_registry(&self, tx: &Transaction<'_>) -> Result<SectionPreview> {
        self.registry_preview(&self.plan(tx).await?)
    }

    fn registry_preview(&self, plan: &PortableRegistryPlan) -> Result<SectionPreview> {
        Ok(SectionPreview {
            needs_apply: plan.needs_apply(),
            changes: self.changes(plan)?,
            blockers: vec![],
        })
    }

    /// Public task rows deliberately omit target-registry IDs. These task
    /// records are visible to tasks-read users, not just messaging admins.
    pub(crate) fn changes(&self, plan: &PortableRegistryPlan) -> Result<Value> {
        self.definitions
            .accounts
            .iter()
            .map(|definition| {
                let reference = definition.reference_key();
                let binding = plan
                    .bindings
                    .get(&reference)
                    .ok_or_else(|| anyhow!("{STALE}"))?;
                let provider = definition.account_sender().provider();
                let mut row = json!({
                    "reference": reference,
                    "provider": provider,
                    "channel": provider.channel(),
                    "action": binding.action,
                });
                // Only which kind of credential is delivered: never a value,
                // ID or digest.
                if definition.credential_commitment.is_some() {
                    row["credentials"] = match definition.sender.delivered_credential() {
                        Some(DeliveredCredentialKind::API_KEY) => json!("API_KEY"),
                        _ => json!("AWS_STATIC_KEY_PAIR"),
                    };
                }
                Ok(row)
            })
            .collect::<Result<Vec<_>>>()
            .map(Value::Array)
    }

    /// Target UUIDs are only for a separately authorized audit record, never
    /// for Preview, task detail/history, or the package itself.
    pub(crate) fn audit_bindings(&self) -> Result<BTreeMap<&str, Uuid>> {
        let plan = self.snapshot.get().ok_or_else(|| anyhow!("{STALE}"))?;
        Ok(plan
            .bindings
            .iter()
            .map(|(name, binding)| (name.as_str(), binding.account_id))
            .collect())
    }

    /// Installs the reviewed accounts, as checked as their package declares
    /// them, and their delivered credentials, all in `tx`. No provider is
    /// asked: an administrator may check an account again at any time.
    pub(crate) async fn install(
        &self,
        tx: &Transaction<'_>,
    ) -> Result<(Value, PortableRegistryPlan)> {
        ensure!(self.confirmed.is_some(), "{STALE}");
        let planned = self.snapshot.get().ok_or_else(|| anyhow!("{STALE}"))?;
        if let Some(document) = self.staged_document {
            package_credentials::lock_for_apply(tx, &self.tenant, &document).await?;
        }
        let credentials = self.credentials(tx, planned).await?;
        let installed = portable_registry::apply(
            tx,
            self.tenant,
            &self.digest,
            &self.definitions,
            planned,
            credentials.as_ref(),
            &self.declared,
        )
        .await?;
        // Same transaction: the staged credentials go only once they are in.
        if let (Some(_), Some(document)) = (&credentials, &self.staged_document) {
            package_credentials::discard(tx, &self.tenant, document).await?;
        }
        debug_assert!(!installed.needs_apply());
        Ok((self.changes(planned)?, installed))
    }

    /// The caller publishes this evidence only after the transaction commits.
    pub(crate) fn installed(&self, installed: PortableRegistryPlan) {
        self.installed_here
            .set(installed)
            .expect("registry installed once");
    }

    pub(crate) async fn preview_event(
        &self,
        tx: &Transaction<'_>,
        value: &Value,
        replacements: &HashMap<String, String>,
        elections: &[String],
    ) -> Result<PreparedMessaging> {
        let plan = self.plan(tx).await?;
        self.prepare_event(tx, &plan, value, replacements, elections)
            .await
    }

    /// Mandatory under the same transaction as the native event import,
    /// after signature admission and before the event realm/boards are made.
    pub(crate) async fn import_event(
        &self,
        tx: &Transaction<'_>,
        tenant: Uuid,
        value: &Value,
        replacements: &HashMap<String, String>,
        elections: &[String],
    ) -> Result<PreparedMessaging> {
        ensure!(tenant == self.tenant && self.confirmed.is_some(), "{STALE}");
        let reviewed = self
            .installed_here
            .get()
            .or_else(|| self.snapshot.get())
            .ok_or_else(|| anyhow!("{STALE}"))?;
        let plan = portable_registry::require_installed(
            tx,
            self.tenant,
            &self.digest,
            &self.definitions,
            reviewed,
        )
        .await?;
        self.prepare_event(tx, &plan, value, replacements, elections)
            .await
    }

    async fn prepare_event(
        &self,
        tx: &Transaction<'_>,
        plan: &PortableRegistryPlan,
        value: &Value,
        replacements: &HashMap<String, String>,
        elections: &[String],
    ) -> Result<PreparedMessaging> {
        let original = with_original_account_ids(value, replacements);
        let original_typed: EventMessagingConfig = serde_json::from_value(original.clone())
            .map_err(|_| anyhow!("Unreadable signed event messaging configuration."))?;
        validate_event_references(Some(&self.definitions), Some(&original_typed))
            .map_err(|_| anyhow!("The signed event does not use exactly its declared package messaging references."))?;
        let portable_channels = original_typed
            .channels
            .iter()
            .filter(|channel| channel.account_id.starts_with(REF_PREFIX))
            .map(|channel| channel.channel)
            .collect();
        let bound = plan.bind(&original)?;
        let accounts = portable_registry::account_summaries(
            tx,
            self.tenant,
            &self.definitions,
            plan,
            &self.declared,
        )
        .await?;
        let config = serde_json::from_value(bound)
            .map_err(|_| anyhow!("Unreadable resolved event messaging configuration."))?;
        prepare(
            config,
            &self.tenant.to_string(),
            elections,
            &accounts,
            &portable_channels,
        )
    }
}

/// The signed settings are kept exactly as signed: channels, purposes,
/// template settings and their buttons. A package's accounts arrive checked
/// as their author checked them; whatever is still not ready on them (a
/// purpose without an approved template, a missing callback address) is
/// reported, never removed, and sends through it wait until it is ready.
/// Every other problem (a missing or foreign account, a wrong channel, a
/// button without a value, a removed provider) still refuses the import.
fn prepare(
    config: EventMessagingConfig,
    tenant: &str,
    elections: &[String],
    accounts: &[AccountSummary],
    portable: &BTreeSet<MessageChannel>,
) -> Result<PreparedMessaging> {
    let readiness = ProviderReadiness::AWAITED(portable.clone());
    let mut pending: BTreeMap<MessageChannel, BTreeSet<MessagePurpose>> = BTreeMap::new();
    let mut refused = vec![];
    for error in config
        .validate(tenant, accounts, elections)
        .err()
        .unwrap_or_default()
    {
        match readiness.awaited(&error) {
            Some((channel, purpose)) => {
                pending.entry(channel).or_default().insert(purpose);
            }
            None => refused.push(explain(&error)),
        }
    }
    ensure!(refused.is_empty(), "{}", refused.join("; "));
    Ok(PreparedMessaging {
        config: serde_json::to_value(config)?,
        pending: pending
            .into_iter()
            .map(|(channel, purposes)| PendingMessaging { channel, purposes })
            .collect(),
        readiness,
    })
}

pub(crate) fn explain(error: &MessagingConfigError) -> String {
    use MessagingConfigError::*;
    match error {
        UNKNOWN_ACCOUNT { channel, .. } => {
            format!("{channel}: the referenced sending account is missing in this tenant")
        }
        ACCOUNT_OF_ANOTHER_TENANT { .. } => {
            "The referenced sending account belongs to a different tenant".into()
        }
        ACCOUNT_CHANNEL_MISMATCH { channel, .. } => {
            format!("{channel}: the referenced sending account is for another channel")
        }
        PURPOSE_NOT_READY {
            channel, purpose, ..
        } => format!("{channel} {purpose}: the sending account is not ready"),
        TEMPLATE_NOT_APPROVED {
            channel, purpose, ..
        } => format!("{channel} {purpose}: a required template is not approved"),
        TEMPLATE_BUTTON_WITHOUT_VALUE {
            channel, purpose, ..
        } => format!("{channel} {purpose}: a template button has no value"),
        OUT_OF_WINDOW_NOT_SUPPORTED { channel } => {
            format!("{channel}: the out-of-window setting is not supported")
        }
        UTILITY_TEMPLATE_MISSING { channel } => {
            format!("{channel}: notices require a bound utility template")
        }
        DUPLICATE_CHANNEL { channel } => {
            format!("{channel}: the configuration lists this channel more than once")
        }
        ELECTION_CHANNEL_NOT_ENABLED { channel, .. } => {
            format!("{channel}: an election selects an undefined channel")
        }
        UNKNOWN_ELECTION { .. } => "The messaging restrictions name an unknown election".into(),
        UNSUPPORTED_VERSION { .. } => "Unsupported event messaging configuration version".into(),
    }
}

/// Preview's note: what the package's accounts aren't ready for yet, as the
/// package declares them.
pub(crate) fn preview_note(pending: &[PendingMessaging]) -> String {
    format!(
        "Not ready yet: {}. These settings are kept as configured; messages on them wait until the accounts, templates or callback address they need are ready.",
        pending_channels(pending)
    )
}

pub(crate) fn pending_note(pending: &[PendingMessaging]) -> String {
    format!(
        "Kept as configured but not ready yet: {}. Messages on them wait until it is: see the event's Messaging tab for what is missing.",
        pending_channels(pending)
    )
}

fn pending_channels(pending: &[PendingMessaging]) -> String {
    pending
        .iter()
        .map(|row| {
            format!(
                "{} ({})",
                row.channel,
                row.purposes
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::election_config::portable_messaging;
    use sequent_core::types::messaging::{AccountCheck, ProviderApproval, ReadinessPolicy};

    const TENANT: &str = "20000000-0000-4000-8000-000000000001";

    fn example() -> (
        EventMessagingConfig,
        Vec<AccountSummary>,
        BTreeSet<MessageChannel>,
    ) {
        let definitions = portable_messaging::parse(&serde_json::to_vec(&json!({
            "format": "sequent.tenant-messaging-accounts/1",
            "accounts": [
                {"reference":"email","name":"Demo email","sender":{"provider":"AWS_SES","from_address":"demo@example.invalid","region":"us-east-1"}},
                {"reference":"sms","name":"Demo SMS","sender":{"provider":"AWS_SNS","sender_id":"DEMO","region":"us-east-1"}},
                {"reference":"viber","name":"Demo Viber","sender":{"provider":"VIBER_INFOBIP","base_url":"https://demo.api.infobip.com","sender":"Demo"}},
                {"reference":"whatsapp","name":"Demo WhatsApp","sender":{"provider":"WHATSAPP_CLOUD_API","business_account_id":"12345","phone_number_id":"12346","display_phone_number":"+12025550198","api_version":"v23.0"}},
                {"reference":"messenger","name":"Demo Messenger","sender":{"provider":"MESSENGER_SEND_API","page_id":"12347","api_version":"v23.0"}}
            ]
        })).unwrap()).unwrap();
        let mut accounts = vec![];
        let mut channels = vec![];
        for (index, definition) in definitions.accounts.iter().enumerate() {
            let sender = definition.account_sender();
            let provider = sender.provider();
            let channel = provider.channel().unwrap();
            let id = format!("10000000-0000-4000-8000-{index:012x}");
            accounts.push(AccountSummary {
                id: id.clone(),
                tenant_id: TENANT.into(),
                channel,
                provider,
                capabilities: sender.capabilities(channel).unwrap(),
                provider_approval: ProviderApproval::PENDING,
                readiness: ReadinessPolicy::PROVIDER_CHECK,
                check: AccountCheck::default(),
                identity: Some(sender.identity()),
                public_label: None,
                messenger_page: None,
                callback_blockers: vec![ReadinessBlocker::CALLBACK_NOT_CONFIGURED],
            });
            let mut row = json!({"channel":channel,"account_id":id,"purposes":["OTP","NOTICE"]});
            if channel == MessageChannel::MESSENGER {
                row["templates"] =
                    json!([{"purpose":"NOTICE","language":"en","provider_template":"demo_notice"}]);
            }
            channels.push(row);
        }
        let portable = accounts.iter().map(|a| a.channel).collect();
        let config = serde_json::from_value(json!({"version":1,"channels":channels})).unwrap();
        (config, accounts, portable)
    }

    #[test]
    fn task_preview_and_apply_never_expose_registry_ids() {
        use crate::services::import::configuration_package_apply::{
            SectionPreview, SectionSubmission,
        };
        use crate::services::messaging::portable_registry::PortableRegistryBinding;
        let definitions = portable_messaging::parse(
            br#"{"format":"sequent.tenant-messaging-accounts/1","accounts":[{"reference":"email","name":"Demo email","sender":{"provider":"AWS_SES","from_address":"demo@example.invalid","region":"us-east-1"}}]}"#
        ).unwrap();
        let tenant = Uuid::parse_str(TENANT).unwrap();
        let digest = "ab".repeat(32);
        let id = Uuid::parse_str("10000000-0000-4000-8000-0123456789ab").unwrap();
        let plan = PortableRegistryPlan {
            tenant_id: tenant,
            package_sha256: digest.clone(),
            bindings: BTreeMap::from([(
                "package:email".into(),
                PortableRegistryBinding {
                    account_id: id,
                    action: RegistryAction::Create,
                    existing_fingerprint: None,
                },
            )]),
        };
        let context = PackageMessaging::new(
            tenant,
            digest.clone(),
            definitions,
            None,
            None,
            None,
            &Default::default(),
        );
        context.snapshot.set(plan.clone()).unwrap();
        let changes = context.changes(&plan).unwrap();
        assert_eq!(
            changes,
            json!([{"reference":"package:email","provider":"AWS_SES","channel":"EMAIL","action":"create"}])
        );
        assert!(!context.needs_credentials(&plan));
        // A committed pair shows only its kind, and a new account needs it.
        let commitment = "cd".repeat(32);
        let committed = portable_messaging::parse(format!(
            r#"{{"format":"sequent.tenant-messaging-accounts/1","accounts":[{{"reference":"email","name":"Demo email","sender":{{"provider":"AWS_SES","from_address":"demo@example.invalid","region":"us-east-1"}},"credential_commitment":"{commitment}"}}]}}"#
        ).as_bytes()).unwrap();
        let with_keys = PackageMessaging::new(
            tenant,
            digest,
            committed,
            None,
            Some(Uuid::from_u128(7)),
            Some("user-a".into()),
            &Default::default(),
        );
        let rows = with_keys.changes(&plan).unwrap();
        assert_eq!(rows[0]["credentials"], "AWS_STATIC_KEY_PAIR");
        assert!(!rows.to_string().contains(&commitment));
        assert!(with_keys.needs_credentials(&plan));
        let mut resumed = plan.clone();
        let existing = resumed.bindings.get_mut("package:email").unwrap();
        existing.existing_fingerprint = Some("ef".repeat(32));
        existing.action = RegistryAction::Replace;
        assert!(with_keys.needs_credentials(&resumed));
        let replacement = with_keys.registry_preview(&resumed).unwrap();
        assert!(replacement.needs_apply);
        assert!(replacement.blockers.is_empty());
        assert_eq!(replacement.changes[0]["action"], "replace");
        assert!(!replacement.changes.to_string().contains(&id.to_string()));
        resumed.bindings.get_mut("package:email").unwrap().action = RegistryAction::Reuse;
        assert!(!with_keys.needs_credentials(&resumed));
        let preview = SectionPreview {
            changes: changes.clone(),
            needs_apply: true,
            blockers: vec![],
        };
        let applied = SectionSubmission {
            task_id: None,
            detail: json!({"changes": changes}),
        };
        assert!(!serde_json::to_string(&preview)
            .unwrap()
            .contains(&id.to_string()));
        assert!(!serde_json::to_string(&applied)
            .unwrap()
            .contains(&id.to_string()));
        assert_eq!(context.audit_bindings().unwrap()["package:email"], id);
    }

    /// Nothing the package's author configured is turned off or dropped
    /// for accounts that aren't ready here yet: it is kept as signed, and
    /// what isn't ready is reported.
    #[test]
    fn a_pristine_tenant_keeps_every_link_purpose_and_template_as_signed() {
        let (intended, accounts, portable) = example();
        let prepared = prepare(intended.clone(), TENANT, &[], &accounts, &portable).unwrap();
        assert_eq!(prepared.pending.len(), 5);
        assert!(prepared.pending.iter().all(|item| item.purposes.len() == 2));
        assert_eq!(prepared.readiness, ProviderReadiness::AWAITED(portable));
        let saved: EventMessagingConfig = serde_json::from_value(prepared.config).unwrap();
        assert_eq!(saved, intended);
    }

    #[test]
    fn an_existing_verified_account_keeps_its_purposes_but_legacy_missing_refs_still_block() {
        let (intended, mut accounts, portable) = example();
        let email = accounts
            .iter_mut()
            .find(|a| a.channel == MessageChannel::EMAIL)
            .unwrap();
        email.provider_approval = ProviderApproval::CONFIRMED;
        email.check.connected = true;
        email.check.production_access = true;
        email.check.checked_at = Some("2026-11-01T00:00:00Z".into());
        let prepared = prepare(intended.clone(), TENANT, &[], &accounts, &portable).unwrap();
        assert_eq!(prepared.pending.len(), 4);
        assert!(prepared
            .pending
            .iter()
            .all(|item| item.channel != MessageChannel::EMAIL));
        let saved: EventMessagingConfig = serde_json::from_value(prepared.config).unwrap();
        assert_eq!(saved, intended);
        let problem = prepare(intended, TENANT, &[], &[], &BTreeSet::new())
            .err()
            .unwrap();
        assert!(problem
            .to_string()
            .contains("the referenced sending account is missing in this tenant"));
    }

    /// The accounts as Apply's own check leaves them: connected, and for
    /// WhatsApp the approved templates the number's listing named.
    fn checked_infobip() -> (Vec<AccountSummary>, Value) {
        let definitions = portable_messaging::parse(&serde_json::to_vec(&json!({
            "format": "sequent.tenant-messaging-accounts/1",
            "accounts": [
                {"reference":"whatsapp","name":"Demo WhatsApp","sender":{"provider":"WHATSAPP_INFOBIP",
                    "base_url":"https://demo.api.infobip.com","sender_number":"447796344125"}},
                {"reference":"viber","name":"Demo Viber","sender":{"provider":"VIBER_INFOBIP",
                    "base_url":"https://demo.api.infobip.com","sender":"Demo","content_mode":"FREE_TEXT"}}
            ]
        })).unwrap()).unwrap();
        let mut accounts = vec![];
        let mut channels = vec![];
        for (index, definition) in definitions.accounts.iter().enumerate() {
            let sender = definition.account_sender();
            let provider = sender.provider();
            let channel = provider.channel().unwrap();
            let id = format!("10000000-0000-4000-8000-{index:012x}");
            let mut check = AccountCheck {
                connected: true,
                production_access: true,
                checked_at: Some("2026-11-27T00:00:00Z".into()),
                ..Default::default()
            };
            check.details.identity = Some(sender.identity());
            let mut row = json!({"channel":channel,"account_id":id,"purposes":["OTP","NOTICE"]});
            if channel == MessageChannel::WHATSAPP {
                check.approved_templates = BTreeMap::from([
                    (MessagePurpose::OTP, vec!["en".to_string()]),
                    (MessagePurpose::NOTICE, vec!["en".to_string()]),
                ]);
                row["templates"] = json!([
                    {"purpose":"OTP","key":"otp","language":"en","provider_language":"en_GB",
                     "provider_template":"otp_code","buttons":[{"type":"URL","parameter":"{{code}}"}]},
                    {"purpose":"NOTICE","language":"en","provider_template":"election_notice"}
                ]);
            }
            accounts.push(AccountSummary {
                id: id.clone(),
                tenant_id: TENANT.into(),
                channel,
                provider,
                capabilities: sender.capabilities(channel).unwrap(),
                provider_approval: ProviderApproval::PENDING,
                readiness: ReadinessPolicy::PROVIDER_CHECK,
                check,
                identity: Some(sender.identity()),
                public_label: None,
                messenger_page: None,
                callback_blockers: vec![ReadinessBlocker::CALLBACK_NOT_CONFIGURED],
            });
            channels.push(row);
        }
        (accounts, json!({"version":1,"channels":channels}))
    }

    #[test]
    fn infobip_settings_stay_as_signed_whether_or_not_their_accounts_are_ready() {
        let (accounts, config) = checked_infobip();
        let portable = accounts.iter().map(|a| a.channel).collect();
        let intended: EventMessagingConfig = serde_json::from_value(config).unwrap();
        let prepared = prepare(intended.clone(), TENANT, &[], &accounts, &portable).unwrap();
        assert_eq!(prepared.pending, vec![]);
        let saved: EventMessagingConfig = serde_json::from_value(prepared.config).unwrap();
        assert_eq!(saved, intended);
        // Not ready (say the WhatsApp templates weren't declared): still
        // every purpose, template, language code and button, as signed.
        let unchecked: Vec<_> = accounts
            .into_iter()
            .map(|mut account| {
                account.check = AccountCheck::default();
                account
            })
            .collect();
        let prepared = prepare(intended.clone(), TENANT, &[], &unchecked, &portable).unwrap();
        assert_eq!(prepared.pending.len(), 2);
        let saved: EventMessagingConfig = serde_json::from_value(prepared.config).unwrap();
        assert_eq!(saved, intended);
        let binding = saved
            .template_for(
                MessageChannel::WHATSAPP,
                MessagePurpose::OTP,
                Some("otp"),
                Some("en"),
            )
            .unwrap();
        assert_eq!(binding.provider_language(), "en_GB");
        assert_eq!(binding.buttons.as_ref().unwrap()[0].parameter, "{{code}}");
    }

    #[test]
    fn a_problem_that_never_gets_ready_still_refuses_the_import() {
        let (accounts, config) = checked_infobip();
        let portable = accounts.iter().map(|a| a.channel).collect();
        let mut intended: EventMessagingConfig = serde_json::from_value(config).unwrap();
        intended.channels[0].templates[0].buttons.as_mut().unwrap()[0].parameter = " ".into();
        let refused = prepare(intended, TENANT, &[], &accounts, &portable)
            .err()
            .unwrap();
        assert!(
            refused
                .to_string()
                .contains("a template button has no value"),
            "{refused}"
        );
    }

    #[test]
    fn notes_say_what_is_not_ready_and_that_it_is_kept() {
        let pending = [PendingMessaging {
            channel: MessageChannel::WHATSAPP,
            purposes: BTreeSet::from([MessagePurpose::OTP, MessagePurpose::NOTICE]),
        }];
        let note = preview_note(&pending);
        assert!(
            note.starts_with("Not ready yet: WHATSAPP (OTP, NOTICE)."),
            "{note}"
        );
        assert!(note.contains("kept as configured"), "{note}");
        assert!(pending_note(&pending).contains("WHATSAPP (OTP, NOTICE)"));
    }
}
