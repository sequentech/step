// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Receiver-owned planning for a signed package's public sending accounts.
//! The signed bytes are never rewritten. The import resolves them in its own
//! transaction to exactly the target accounts the operator previewed.

use crate::services::import::configuration_package_apply::SectionPreview;
use crate::services::import::configuration_package_credentials as package_credentials;
use crate::services::messaging::config::with_original_account_ids;
use crate::services::messaging::portable_registry::{self, PortableRegistryPlan, RegistryAction};
use anyhow::{anyhow, ensure, Result};
use deadpool_postgres::Transaction;
use sequent_core::election_config::portable_messaging::{
    validate_event_references, PortableMessagingAccounts, REF_PREFIX,
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
    /// The protected staging document whose vault entry holds any AWS key
    /// pairs delivered with this package. Set by the server, never the package.
    staged_document: Option<Uuid>,
    /// The task's verified actor (the JWT-derived audit user ID Harvest put
    /// in the task body): staged credentials are only for their uploader.
    actor: Option<String>,
    snapshot: OnceLock<PortableRegistryPlan>,
    installed_here: OnceLock<PortableRegistryPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PendingMessaging {
    pub channel: MessageChannel,
    pub purposes: BTreeSet<MessagePurpose>,
}

pub(crate) struct PreparedMessaging {
    pub config: Value,
    pub pending: Vec<PendingMessaging>,
}

impl PackageMessaging {
    pub(crate) fn new(
        tenant: Uuid,
        digest: String,
        definitions: PortableMessagingAccounts,
        confirmed: Option<String>,
        staged_document: Option<Uuid>,
        actor: Option<String>,
    ) -> Self {
        Self {
            tenant,
            digest,
            definitions,
            confirmed,
            staged_document,
            actor,
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
                // Only that keys are delivered: never a value, ID or digest.
                if definition.credential_commitment.is_some() {
                    row["credentials"] = json!("AWS_STATIC_KEY_PAIR");
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
        )
        .await?;
        // Same transaction: the staged pair goes only once its keys are in.
        if let (Some(_), Some(document)) = (&credentials, &self.staged_document) {
            package_credentials::discard(tx, &self.tenant, document).await?;
        }
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
        let accounts =
            portable_registry::account_summaries(tx, self.tenant, &self.definitions, plan).await?;
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

/// For a writable public definition, unavailable live credentials/checks do
/// **not** turn into provider approval or public capability. Keep the account
/// link, but leave the affected live purposes disabled. All structural /
/// legacy failures still block. The final shape must pass the usual validator.
fn prepare(
    mut config: EventMessagingConfig,
    tenant: &str,
    elections: &[String],
    accounts: &[AccountSummary],
    portable: &BTreeSet<MessageChannel>,
) -> Result<PreparedMessaging> {
    let mut pending: BTreeMap<MessageChannel, BTreeSet<MessagePurpose>> = BTreeMap::new();
    for error in config
        .validate(tenant, accounts, elections)
        .err()
        .unwrap_or_default()
    {
        match error {
            MessagingConfigError::PURPOSE_NOT_READY {
                channel,
                purpose,
                blockers,
            } if portable.contains(&channel)
                && !blockers.iter().any(|reason| {
                    matches!(
                        reason,
                        ReadinessBlocker::UNSUPPORTED_PROVIDER
                            | ReadinessBlocker::UNSUPPORTED_PURPOSE
                    )
                }) =>
            {
                pending.entry(channel).or_default().insert(purpose);
            }
            MessagingConfigError::TEMPLATE_NOT_APPROVED {
                channel, purpose, ..
            } if portable.contains(&channel) => {
                pending.entry(channel).or_default().insert(purpose);
            }
            other => return Err(anyhow!("{}", explain(&other))),
        }
    }
    for channel in &mut config.channels {
        if let Some(purposes) = pending.get(&channel.channel) {
            channel
                .purposes
                .retain(|purpose| !purposes.contains(purpose));
            channel
                .templates
                .retain(|template| !purposes.contains(&template.purpose));
        }
    }
    if let Err(errors) = config.validate(tenant, accounts, elections) {
        return Err(anyhow!(
            "{}",
            errors.iter().map(explain).collect::<Vec<_>>().join("; ")
        ));
    }
    Ok(PreparedMessaging {
        config: serde_json::to_value(config)?,
        pending: pending
            .into_iter()
            .map(|(channel, purposes)| PendingMessaging { channel, purposes })
            .collect(),
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

pub(crate) fn pending_note(pending: &[PendingMessaging]) -> String {
    let channels = pending
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
        .collect::<Vec<_>>();
    format!(
        "Sending is off for {}. Configure the destination credentials, check the accounts and templates, then enable the purposes in the event.",
        channels.join("; ")
    )
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
        let context = PackageMessaging::new(tenant, digest.clone(), definitions, None, None, None);
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

    #[test]
    fn a_pristine_tenant_retains_all_five_links_but_offers_no_unchecked_otp_or_notice() {
        let (intended, accounts, portable) = example();
        let prepared = prepare(intended.clone(), TENANT, &[], &accounts, &portable).unwrap();
        assert_eq!(prepared.pending.len(), 5);
        assert!(prepared.pending.iter().all(|item| item.purposes.len() == 2));
        let saved: EventMessagingConfig = serde_json::from_value(prepared.config).unwrap();
        assert_eq!(saved.channels.len(), 5);
        for (current, source) in saved.channels.iter().zip(&intended.channels) {
            assert_eq!(current.account_id, source.account_id);
            assert!(current.purposes.is_empty());
        }
        assert!(saved.enabled_channels(MessagePurpose::OTP, None).is_empty());
        assert!(saved
            .enabled_channels(MessagePurpose::NOTICE, None)
            .is_empty());
        let _ = saved.validate(TENANT, &accounts, &[]).unwrap();
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
        let saved: EventMessagingConfig = serde_json::from_value(prepared.config).unwrap();
        assert_eq!(
            saved.enabled_channels(MessagePurpose::OTP, None),
            [MessageChannel::EMAIL]
        );
        assert_eq!(
            saved.enabled_channels(MessagePurpose::NOTICE, None),
            [MessageChannel::EMAIL]
        );
        let problem = prepare(intended, TENANT, &[], &[], &BTreeSet::new())
            .err()
            .unwrap();
        assert!(problem
            .to_string()
            .contains("the referenced sending account is missing in this tenant"));
    }
}
