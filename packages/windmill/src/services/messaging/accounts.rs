// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Sending accounts: credentials, connection checks and the callback URLs
//! administrators enter at the provider. Callbacks reach Step only through
//! the webhook component (`messaging-webhooks`), never harvest directly.

use super::keys::{credential_secret_key, tenant_key, TenantKey};
use crate::postgres::messaging::{
    delete_messaging_account, empty_status, event_configs_using_account, get_messaging_account,
    record_credentials_replaced, store_messaging_account_check, MessagingAccount,
};
use crate::services::vault::{delete_secret, read_secret, replace_secret};
use anyhow::{anyhow, Result};
use chrono::Utc;
use deadpool_postgres::Transaction;
use messaging::destination::Destination;
use messaging::providers::{
    self, meta_send_scope, Account, Endpoints, APP_SECRET_REFUSED, TOKEN_CANNOT_SEND,
    TOKEN_INVALID, TOKEN_NOT_CHECKED,
};
use messaging::relay::{CallbackProvider, WEBHOOKS_PUBLIC_URL_ENV};
use messaging::webhooks::NumberChange as NumberChangeSignal;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::keycloak::MESSENGER_PAGE_ATTR_NAME;
use sequent_core::types::messaging::{
    purpose_readiness, AccountCheck, AccountSender, CallbackStatus, CredentialName,
    CredentialRecord, MessageChannel, MessagePurpose, MessagingProvider, NumberChange,
    ProviderSupport, ReadinessBlocker, NUMBER_CHANGES_KEPT,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use strum::IntoEnumIterator;
use tracing::{info, warn};
use uuid::Uuid;

/// Why an account has, or lacks, a callback URL. Computed only by
/// [`callback_readiness`]; the account check, the Admin Portal and the logs
/// all show its reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackReadiness {
    /// The provider sends no callbacks to Step (SMS through SNS, SMTP).
    NotUsed,
    /// The URL the administrator enters at the provider, on the webhook
    /// component.
    Ready(String),
    /// `MESSAGING_WEBHOOKS_PUBLIC_URL` is unset. Delivery receipts are
    /// unavailable (never shown as "0 delivered"), and Messenger can't take
    /// codes, because they need Get Started.
    NotConfigured,
    /// The URL, on an address that isn't `https`. Meta refuses such
    /// callback URLs; only a development stack, whose stand-in provider
    /// calls the component inside its network, can use it. Shown with its
    /// reason, never as ready.
    Insecure(String),
}

/// The reason of [`CallbackReadiness::NotConfigured`].
pub const CALLBACK_NOT_CONFIGURED: &str = "Callback address not configured";

/// The reason of [`CallbackReadiness::Insecure`].
pub const CALLBACK_NOT_HTTPS: &str = "Callback address must be https";

impl CallbackReadiness {
    /// The callback URL, when there is one.
    pub fn url(&self) -> Option<&str> {
        match self {
            Self::Ready(url) | Self::Insecure(url) => Some(url),
            Self::NotUsed | Self::NotConfigured => None,
        }
    }

    /// Why there is no usable callback URL, when the provider needs one.
    pub fn reason(&self) -> Option<&'static str> {
        match self {
            Self::NotConfigured => Some(CALLBACK_NOT_CONFIGURED),
            Self::Insecure(_) => Some(CALLBACK_NOT_HTTPS),
            Self::NotUsed | Self::Ready(_) => None,
        }
    }
}

/// The reason of [`ReadinessBlocker::REPORT_CREDENTIALS_MISSING`].
pub const REPORT_CREDENTIALS_MISSING: &str =
    "Generate the report credentials and create the Infobip subscription";

/// The one reason of a callback blocker, as the UI and the log show it.
pub fn callback_blocker_reason(blocker: ReadinessBlocker) -> Option<&'static str> {
    match blocker {
        ReadinessBlocker::CALLBACK_NOT_CONFIGURED => Some(CALLBACK_NOT_CONFIGURED),
        ReadinessBlocker::REPORT_CREDENTIALS_MISSING => Some(REPORT_CREDENTIALS_MISSING),
        _ => None,
    }
}

/// The start of the one reason a send through an account that isn't ready
/// is refused with; [`not_ready_reason`] adds the blocker's code.
pub const NOT_READY_REASON: &str = "Not sent: the account isn't ready";

/// The reason a send refused by [`send_readiness`] is recorded with, for
/// the ledger, the log and the Admin Portal (which translates the blocker):
/// `"Not sent: the account isn't ready (NOT_CONNECTED)"`.
pub fn not_ready_reason(blocker: ReadinessBlocker) -> String {
    format!("{NOT_READY_REASON} ({blocker})")
}

/// Whether `account` may send `purpose` now: the same readiness the event
/// configuration is saved with ([`purpose_readiness`]), for the account's
/// current identity. A check of another identity (an earlier number, Page
/// or sender) says nothing about this one ([`ReadinessBlocker::UNCHECKED`]);
/// an account never checked isn't connected. The first blocker is the
/// refusal ([`not_ready_reason`]). Templates count in any language here;
/// the send itself picks the voter's.
pub fn send_readiness(
    account: &MessagingAccount,
    purpose: MessagePurpose,
) -> std::result::Result<(), ReadinessBlocker> {
    if account.provider.support() == ProviderSupport::UNSUPPORTED {
        return Err(ReadinessBlocker::UNSUPPORTED_PROVIDER);
    }
    let Some(capabilities) = account.sender.capabilities(account.channel) else {
        return Err(ReadinessBlocker::UNSUPPORTED_PURPOSE);
    };
    let readiness = purpose_readiness(
        &capabilities,
        account.provider_approval,
        Some(&account.sender.identity()),
        &account.status,
        purpose,
        None,
    );
    match readiness.blockers.first() {
        Some(blocker) => Err(*blocker),
        None => Ok(()),
    }
}

/// What stops the account's callbacks from reaching Step, for a given
/// public base URL: the callback address ([`callback_readiness_with`]) and,
/// for Viber, the report credentials its Infobip subscription sends, read
/// from the vault's credential records. Receipts are unavailable while any
/// is set; Messenger isn't ready for codes without the address.
pub fn callback_blockers_with(
    component_url: Option<&str>,
    account: &MessagingAccount,
) -> Vec<ReadinessBlocker> {
    let mut blockers = vec![];
    if callback_readiness_with(component_url, account.provider, &account.webhook_key)
        == CallbackReadiness::NotConfigured
    {
        blockers.push(ReadinessBlocker::CALLBACK_NOT_CONFIGURED);
    }
    if account.provider == MessagingProvider::VIBER_INFOBIP
        && !(account
            .credentials
            .contains_key(&CredentialName::REPORT_USERNAME)
            && account
                .credentials
                .contains_key(&CredentialName::REPORT_PASSWORD))
    {
        blockers.push(ReadinessBlocker::REPORT_CREDENTIALS_MISSING);
    }
    blockers
}

/// [`callback_blockers_with`] on `MESSAGING_WEBHOOKS_PUBLIC_URL`.
pub fn callback_blockers(account: &MessagingAccount) -> Vec<ReadinessBlocker> {
    callback_blockers_with(webhooks_public_url().as_deref(), account)
}

/// The webhook component's public base URL, from
/// `MESSAGING_WEBHOOKS_PUBLIC_URL`.
pub fn webhooks_public_url() -> Option<String> {
    std::env::var(WEBHOOKS_PUBLIC_URL_ENV)
        .ok()
        .map(|url| url.trim().trim_end_matches('/').to_string())
        .filter(|url| !url.is_empty())
}

/// The component route family a provider's callbacks use, if any.
fn callback_provider(provider: MessagingProvider) -> Option<CallbackProvider> {
    match provider {
        MessagingProvider::WHATSAPP_CLOUD_API | MessagingProvider::MESSENGER_SEND_API => {
            Some(CallbackProvider::META)
        }
        MessagingProvider::VIBER_INFOBIP => Some(CallbackProvider::VIBER),
        MessagingProvider::AWS_SES => Some(CallbackProvider::AWS),
        MessagingProvider::AWS_SNS
        | MessagingProvider::SMTP
        | MessagingProvider::HTTP_API
        | MessagingProvider::CONSOLE => None,
    }
}

/// [`callback_readiness`] for a given public base URL.
pub fn callback_readiness_with(
    component_url: Option<&str>,
    provider: MessagingProvider,
    webhook_key: &str,
) -> CallbackReadiness {
    let Some(route) = callback_provider(provider) else {
        return CallbackReadiness::NotUsed;
    };
    match component_url.map(str::trim).filter(|url| !url.is_empty()) {
        Some(base)
            if base
                .get(..8)
                .is_some_and(|s| s.eq_ignore_ascii_case("https://")) =>
        {
            CallbackReadiness::Ready(route.callback_url(base, webhook_key))
        }
        Some(base) => CallbackReadiness::Insecure(route.callback_url(base, webhook_key)),
        None => CallbackReadiness::NotConfigured,
    }
}

/// Where the provider of `account` sends delivery reports and events: the
/// webhook component's route for the provider
/// ([`CallbackProvider::callback_url`] on `MESSAGING_WEBHOOKS_PUBLIC_URL`).
pub fn callback_readiness(account: &MessagingAccount) -> CallbackReadiness {
    callback_readiness_with(
        webhooks_public_url().as_deref(),
        account.provider,
        &account.webhook_key,
    )
}

/// The account's callback URL for its adapter. Says once per process when
/// callbacks are not configured.
fn runtime_callback_url(account: &MessagingAccount) -> Option<String> {
    static WARNED: AtomicBool = AtomicBool::new(false);
    let readiness = callback_readiness(account);
    if let Some(reason) = readiness.reason() {
        if !WARNED.swap(true, Ordering::Relaxed) {
            warn!(provider = %account.provider, "{reason}: delivery reports and Get Started can't reach Step");
        }
    }
    readiness.url().map(str::to_string)
}

/// The account with its credentials decrypted, ready for an adapter.
pub async fn runtime_account(tx: &Transaction<'_>, account: &MessagingAccount) -> Result<Account> {
    let tenant = account.tenant_id.to_string();
    let mut credentials = BTreeMap::new();
    for name in account.provider.accepted_credentials() {
        if let Some(value) =
            read_secret(tx, &tenant, None, &credential_secret_key(&account.id, name)).await?
        {
            credentials.insert(name, value);
        }
    }
    Ok(Account {
        id: account.id.to_string(),
        channel: account.channel,
        sender: account.sender.clone(),
        credentials,
        limits: account.limits.clone(),
        callback_url: runtime_callback_url(account),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReplacedCredentials {
    pub replaced: Vec<CredentialName>,
    /// Only when a new verify token was generated. Shown once.
    pub verify_token: Option<String>,
}

/// Replaces credentials. The values go to the secret store; the account
/// only records when each was replaced.
pub async fn replace_credentials(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    account_id: &Uuid,
    values: &BTreeMap<CredentialName, String>,
    generate_verify_token: bool,
) -> Result<Option<ReplacedCredentials>> {
    let Some(account) = get_messaging_account(tx, tenant_id, account_id).await? else {
        return Ok(None);
    };
    let accepted = account.provider.accepted_credentials();
    let mut to_store = entered_credentials(&account, values)?;
    let verify_token = if generate_verify_token {
        if !accepted.contains(&CredentialName::VERIFY_TOKEN) {
            return Err(anyhow!(
                "{} accounts have no verify token",
                account.provider
            ));
        }
        let token = messaging::link::new_reference();
        to_store.insert(CredentialName::VERIFY_TOKEN, token.clone());
        Some(token)
    } else {
        None
    };
    let tenant = tenant_id.to_string();
    for (name, value) in &to_store {
        replace_secret(
            tx,
            &tenant,
            &credential_secret_key(account_id, *name),
            value,
        )
        .await?;
    }
    let replaced: Vec<CredentialName> = to_store.keys().copied().collect();
    record_credentials_replaced(tx, tenant_id, account_id, &replaced, Utc::now()).await?;
    Ok(Some(ReplacedCredentials {
        replaced,
        verify_token,
    }))
}

/// Typed credentials the account takes, trimmed; refused with the reason
/// when one is generated (never entered), not used by the provider, or
/// empty. Checked before anything else, the provider's check included.
pub fn entered_credentials(
    account: &MessagingAccount,
    values: &BTreeMap<CredentialName, String>,
) -> Result<BTreeMap<CredentialName, String>> {
    let accepted = account.provider.accepted_credentials();
    let mut entered = BTreeMap::new();
    for (name, value) in values {
        if *name == CredentialName::VERIFY_TOKEN {
            return Err(anyhow!("the verify token is generated, not entered"));
        }
        if name.is_generated() {
            return Err(anyhow!("{name} is generated, not entered"));
        }
        if !accepted.contains(name) {
            return Err(anyhow!("{} accounts do not use {name}", account.provider));
        }
        if value.trim().is_empty() {
            return Err(anyhow!("{name} cannot be empty"));
        }
        entered.insert(*name, value.trim().to_string());
    }
    Ok(entered)
}

/// Generates new report credentials for an Infobip Viber account: the Basic
/// auth its report subscription sends. Returned once, to be shown once;
/// only the vault keeps them. The previous ones stop working.
pub async fn generate_report_credentials(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    account_id: &Uuid,
) -> Result<Option<messaging::providers::infobip::ReportCredentials>> {
    let Some(account) = get_messaging_account(tx, tenant_id, account_id).await? else {
        return Ok(None);
    };
    if !account
        .provider
        .accepted_credentials()
        .contains(&CredentialName::REPORT_PASSWORD)
    {
        return Err(anyhow!(
            "{} accounts have no report credentials",
            account.provider
        ));
    }
    let generated = messaging::providers::infobip::new_report_credentials();
    let tenant = tenant_id.to_string();
    for (name, value) in [
        (CredentialName::REPORT_USERNAME, &generated.username),
        (CredentialName::REPORT_PASSWORD, &generated.password),
    ] {
        replace_secret(tx, &tenant, &credential_secret_key(account_id, name), value).await?;
    }
    record_credentials_replaced(
        tx,
        tenant_id,
        account_id,
        &[
            CredentialName::REPORT_USERNAME,
            CredentialName::REPORT_PASSWORD,
        ],
        Utc::now(),
    )
    .await?;
    Ok(Some(generated))
}

/// Why credentials typed to replace stored ones weren't saved: the check
/// with them didn't connect. The stored ones stay in use. Shown after the
/// check's own reason (`"<check reason>; <this>"`).
pub const CANDIDATE_REFUSED_REASON: &str =
    "The new credentials didn't connect: the previous ones are kept";

/// Why an account isn't deleted: an election event's messaging settings
/// send through it.
pub const ACCOUNT_IN_USE_REASON: &str =
    "Used by an election event's messaging settings: remove it there first";

/// What a WhatsApp `user_changed_number` signal means for the voters at
/// the old number, as the log and the account show it.
pub const NUMBER_CHANGED_REASON: &str =
    "A voter changed their WhatsApp number: ask them to verify their contact again";

/// Whether the provider's check says whether this credential works. The
/// verify token only concerns the callback subscription, and the report
/// credentials are Step's own: a check says nothing about them. The app
/// secret is checked with Meta's proof (R5-16).
pub fn checked_by_provider(name: CredentialName) -> bool {
    matches!(
        name,
        CredentialName::ACCESS_TOKEN
            | CredentialName::APP_SECRET
            | CredentialName::API_KEY
            | CredentialName::SMTP_PASSWORD
            | CredentialName::AWS_ACCESS_KEY_ID
            | CredentialName::AWS_SECRET_ACCESS_KEY
            | CredentialName::API_SECRET
            | CredentialName::USERNAME
            | CredentialName::PASSWORD
    )
}

/// Whether typed credentials must pass a check before they are saved: one
/// the provider's check connects with replaces one the account already
/// has. First entries are saved and checked afterwards, since there is
/// nothing to keep (BC-F11).
pub fn candidate_check_needed(
    account: &MessagingAccount,
    values: &BTreeMap<CredentialName, String>,
) -> bool {
    values
        .keys()
        .any(|name| checked_by_provider(*name) && account.credentials.contains_key(name))
}

/// Checks typed credentials before anything is saved: the account's
/// stored credentials with the typed ones in their place, sent as `sender`
/// (the settings being saved with them). Nothing is written.
pub async fn check_candidate(
    tx: &Transaction<'_>,
    account: &MessagingAccount,
    sender: &AccountSender,
    values: &BTreeMap<CredentialName, String>,
    endpoints: &Endpoints,
) -> Result<AccountCheck> {
    let entered = entered_credentials(account, values)?;
    let mut candidate = runtime_account(tx, account).await?;
    candidate.sender = sender.clone();
    candidate.credentials.extend(entered);
    Ok(run_check(&candidate, endpoints).await)
}

/// The refusal of typed credentials, or `None` when their check lets them
/// replace the stored ones: it connected and, for Meta, `debug_token` says
/// the token is valid, not expired and allowed to send (R5-11), and Meta
/// didn't refuse the app secret's proof (R5-16). One string for the
/// route, the log and the Admin Portal: `"<why>; <CANDIDATE_REFUSED>"`.
pub fn candidate_refusal(check: &AccountCheck, provider: MessagingProvider) -> Option<String> {
    let refused = |reason: Option<&str>| match reason {
        Some(reason) if !reason.trim().is_empty() => {
            format!("{reason}; {CANDIDATE_REFUSED_REASON}")
        }
        _ => CANDIDATE_REFUSED_REASON.to_string(),
    };
    if !check.connected {
        return Some(refused(check.reason.as_deref()));
    }
    if let Some(send_scope) = meta_send_scope(provider) {
        let Some(token) = &check.details.token else {
            return Some(refused(Some(TOKEN_NOT_CHECKED)));
        };
        let expired = token
            .expires_at
            .as_deref()
            .and_then(|at| chrono::DateTime::parse_from_rfc3339(at).ok())
            .is_some_and(|at| at <= Utc::now());
        if !token.valid || expired {
            return Some(refused(Some(TOKEN_INVALID)));
        }
        if token.missing_scopes.iter().any(|scope| scope == send_scope) {
            return Some(refused(Some(TOKEN_CANNOT_SEND)));
        }
    }
    if check
        .details
        .app_secret
        .as_ref()
        .is_some_and(|secret| secret.valid == Some(false))
    {
        return Some(refused(Some(APP_SECRET_REFUSED)));
    }
    None
}

/// What happened to credentials typed to replace stored ones.
#[derive(Debug, Clone, PartialEq)]
pub enum CredentialsOutcome {
    /// Saved; with the check that let them through, when one was needed
    /// (it is stored as the account's status).
    Replaced(ReplacedCredentials, Option<AccountCheck>),
    /// Not saved, with [`candidate_refusal`]'s reason. The stored
    /// credentials stay.
    Refused(String),
}

/// [`replace_credentials`] for typed credentials: those that replace
/// stored ones are checked first and saved only when the check connects
/// ([`candidate_check_needed`]). Adapters read credentials from the vault on
/// each use, so a replacement takes effect without a restart.
pub async fn replace_checked_credentials(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    account_id: &Uuid,
    values: &BTreeMap<CredentialName, String>,
    generate_verify_token: bool,
    endpoints: &Endpoints,
) -> Result<Option<CredentialsOutcome>> {
    let Some(account) = get_messaging_account(tx, tenant_id, account_id).await? else {
        return Ok(None);
    };
    let check = if candidate_check_needed(&account, values) {
        let check = check_candidate(tx, &account, &account.sender, values, endpoints).await?;
        if let Some(reason) = candidate_refusal(&check, account.provider) {
            warn!(account = %account.id, "credentials not replaced: {reason}");
            return Ok(Some(CredentialsOutcome::Refused(reason)));
        }
        Some(check)
    } else {
        None
    };
    let Some(replaced) =
        replace_credentials(tx, tenant_id, account_id, values, generate_verify_token).await?
    else {
        return Ok(None);
    };
    if let Some(check) = &check {
        store_candidate_check(tx, &account, check.clone()).await?;
        info!(account = %account.id, "credentials replaced after a check that connected");
    }
    Ok(Some(CredentialsOutcome::Replaced(replaced, check)))
}

/// The provider's check of a runtime account; a sender that can't be built
/// gives a check that says why. The check names the identity it checked
/// and when it started (R5-09).
async fn run_check(account: &Account, endpoints: &Endpoints) -> AccountCheck {
    let started_at = Utc::now().to_rfc3339();
    let failed = |reason: String| AccountCheck {
        checked_at: Some(Utc::now().to_rfc3339()),
        reason: Some(reason),
        ..Default::default()
    };
    let mut check = match providers::http_client() {
        Err(error) => failed(error.to_string()),
        Ok(http) => match providers::build(account, &http, endpoints).await {
            Ok(sender) => sender.check().await,
            Err(error) => failed(error.to_string()),
        },
    };
    check.details.identity = Some(account.sender.identity());
    check.details.started_at = Some(started_at);
    check.details.attempted_at = None;
    check
}

/// A check's result as the account stores it, with what the check itself
/// doesn't read: the callback address (`callback_readiness`). A check
/// that doesn't name its identity is of `account`'s.
fn with_account_details(account: &MessagingAccount, mut check: AccountCheck) -> AccountCheck {
    let callback = callback_readiness(account);
    if callback != CallbackReadiness::NotUsed {
        check.details.callback = Some(CallbackStatus {
            url: callback.url().map(str::to_string),
            reason: callback.reason().map(str::to_string),
        });
    }
    if check.details.identity.is_none() {
        check.details.identity = Some(account.sender.identity());
    }
    check
}

/// Stores a check of `account`, as it was read when the check started, as
/// its status: only if its identity and credentials are still those
/// (R5-03), keeping the number-change signals the row has now (R5-12).
/// `None`: the account changed while it was checked, and the result was
/// dropped and logged.
pub async fn store_check(
    tx: &Transaction<'_>,
    account: &MessagingAccount,
    check: AccountCheck,
) -> Result<Option<AccountCheck>> {
    store(tx, account, check, Some(&account.credentials)).await
}

/// [`store_check`] for a check of credentials saved in the same
/// transaction (a candidate): compared by identity only.
pub async fn store_candidate_check(
    tx: &Transaction<'_>,
    account: &MessagingAccount,
    check: AccountCheck,
) -> Result<Option<AccountCheck>> {
    store(tx, account, check, None).await
}

async fn store(
    tx: &Transaction<'_>,
    account: &MessagingAccount,
    check: AccountCheck,
    credentials: Option<&BTreeMap<CredentialName, CredentialRecord>>,
) -> Result<Option<AccountCheck>> {
    let mut check = with_account_details(account, check);
    let checked = check
        .details
        .identity
        .clone()
        .unwrap_or_else(|| account.sender.identity());
    let stored = store_messaging_account_check(
        tx,
        &account.tenant_id,
        &account.id,
        &checked,
        credentials,
        &check,
    )
    .await?;
    if !stored {
        warn!(account = %account.id, "check not stored: {CHECK_DROPPED_REASON}");
        return Ok(None);
    }
    // What the row holds now: the signals are the row's.
    if let Some(current) = get_messaging_account(tx, &account.tenant_id, &account.id).await? {
        check.details.number_changes = current.status.details.number_changes;
    }
    Ok(Some(check))
}

/// Why a check's result wasn't stored: the account was saved with another
/// identity or other credentials while it was checked (R5-03).
pub const CHECK_DROPPED_REASON: &str = "The account changed while it was checked: check it again";

/// A check's result, and whether it was stored as the account's status.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredCheck {
    pub check: AccountCheck,
    /// `false`: the account's identity or credentials changed while it was
    /// checked, so the result describes what it no longer is (R5-03).
    pub stored: bool,
}

/// Checks the account with its provider and stores the result, unless the
/// account changed while it was checked. The provider is called without a
/// lock held: a save during the check isn't kept waiting, and the result
/// is compared with the row when it is stored.
pub async fn check_account(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    account_id: &Uuid,
    endpoints: &Endpoints,
) -> Result<Option<StoredCheck>> {
    let Some(account) = get_messaging_account(tx, tenant_id, account_id).await? else {
        return Ok(None);
    };
    let runtime = runtime_account(tx, &account).await?;
    let check = run_check(&runtime, endpoints).await;
    Ok(Some(
        match store_check(tx, &account, check.clone()).await? {
            Some(check) => StoredCheck {
                check,
                stored: true,
            },
            None => StoredCheck {
                check,
                stored: false,
            },
        },
    ))
}

/// What deleting an account did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteOutcome {
    Deleted,
    NotFound,
    /// Refused ([`ACCOUNT_IN_USE_REASON`]): these election events' messaging
    /// settings send through it.
    InUse(Vec<Uuid>),
}

/// Deletes the account and its credentials, unless an election event's
/// messaging settings use it.
pub async fn delete_account(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    account_id: &Uuid,
) -> Result<DeleteOutcome> {
    // The row is locked before the references are read: a configuration
    // save that names the account holds a share lock on it until it
    // commits, so either the save waits and then finds no account, or the
    // delete waits and then finds the reference (R5-05).
    let locked = tx
        .query_opt(
            r#"
            SELECT id FROM sequent_backend.messaging_account
            WHERE tenant_id = $1 AND id = $2
            FOR UPDATE
            "#,
            &[tenant_id, account_id],
        )
        .await?;
    if locked.is_none() {
        return Ok(DeleteOutcome::NotFound);
    }
    let events = event_configs_using_account(tx, tenant_id, account_id).await?;
    if !events.is_empty() {
        warn!(
            account = %account_id,
            events = events.len(),
            "account not deleted: {ACCOUNT_IN_USE_REASON}"
        );
        return Ok(DeleteOutcome::InUse(events));
    }
    let tenant = tenant_id.to_string();
    for name in CredentialName::iter() {
        delete_secret(tx, &tenant, &credential_secret_key(account_id, name)).await?;
    }
    Ok(
        match delete_messaging_account(tx, tenant_id, account_id).await? {
            true => DeleteOutcome::Deleted,
            false => DeleteOutcome::NotFound,
        },
    )
}

/// What changing a Messenger account's Page does to its voters: their
/// Page-scoped IDs belong to the old Page, so they must connect again
/// (BC-F13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeImpact {
    /// Voters whose `messenger-page` is the account's current Page, in the
    /// realms of the events whose settings use the account.
    pub affected_voters: i64,
    pub election_event_ids: Vec<Uuid>,
}

/// [`ChangeImpact`] of moving the account to `new_page_id`. Nobody is
/// affected when the account isn't a Messenger one or the Page stays.
pub async fn messenger_page_change_impact(
    hasura: &Transaction<'_>,
    keycloak: &Transaction<'_>,
    tenant_id: &Uuid,
    account_id: &Uuid,
    new_page_id: Option<&str>,
) -> Result<Option<ChangeImpact>> {
    let Some(account) = get_messaging_account(hasura, tenant_id, account_id).await? else {
        return Ok(None);
    };
    let AccountSender::MESSENGER_SEND_API { page_id, .. } = &account.sender else {
        return Ok(Some(ChangeImpact {
            affected_voters: 0,
            election_event_ids: vec![],
        }));
    };
    let events = event_configs_using_account(hasura, tenant_id, account_id).await?;
    if new_page_id.map(str::trim) == Some(page_id.trim()) || events.is_empty() {
        return Ok(Some(ChangeImpact {
            affected_voters: 0,
            election_event_ids: events,
        }));
    }
    let tenant = tenant_id.to_string();
    let realms: Vec<String> = events
        .iter()
        .map(|event| get_event_realm(&tenant, &event.to_string()))
        .collect();
    let row = keycloak
        .query_one(
            r#"
            SELECT COUNT(DISTINCT ua.user_id) AS voters
            FROM user_attribute ua
            JOIN user_entity ue ON ue.id = ua.user_id
            JOIN realm r ON r.id = ue.realm_id
            WHERE r.name = ANY($1) AND ua.name = $2 AND ua.value = $3
            "#,
            &[&realms, &MESSENGER_PAGE_ATTR_NAME, &page_id.trim()],
        )
        .await?;
    Ok(Some(ChangeImpact {
        affected_voters: row.try_get("voters")?,
        election_event_ids: events,
    }))
}

/// How many voters a number-change signal may name.
const NUMBER_CHANGE_VOTERS: i64 = 20;

/// Records a WhatsApp `user_changed_number` signal on the account: the
/// voters the account sent to at the old number (found by the ledger's
/// keyed digest, never by the number) are flagged for verification again,
/// shown on the account and logged. Nothing changes automatically: no
/// voter's contact is replaced or removed (BC-F19). `None` when the old
/// number isn't readable.
pub async fn flag_number_change(
    tx: &Transaction<'_>,
    account: &MessagingAccount,
    signal: &NumberChangeSignal,
) -> Result<Option<NumberChange>> {
    let phone = |digits: &str| {
        let digits = digits.trim();
        let number = if digits.starts_with('+') {
            digits.to_string()
        } else {
            format!("+{digits}")
        };
        Destination::parse(MessageChannel::WHATSAPP, &number).ok()
    };
    let Some(old) = phone(&signal.old_number) else {
        warn!(account = %account.id, "WhatsApp number change ignored: unreadable old number");
        return Ok(None);
    };
    let key = tenant_key(tx, &account.tenant_id, TenantKey::Destination).await?;
    let voter_ids: Vec<String> = tx
        .query(
            r#"
            SELECT voter_id
            FROM sequent_backend.message
            WHERE tenant_id = $1 AND account_id = $2 AND destination_digest = $3
              AND voter_id IS NOT NULL
            GROUP BY voter_id
            ORDER BY max(created_at) DESC
            LIMIT $4
            "#,
            &[
                &account.tenant_id,
                &account.id,
                &old.digest(&key)?,
                &NUMBER_CHANGE_VOTERS,
            ],
        )
        .await?
        .iter()
        .map(|row| row.try_get("voter_id"))
        .collect::<std::result::Result<_, _>>()?;
    let change = NumberChange {
        masked_old_number: old.masked(),
        masked_new_number: signal
            .new_number
            .as_deref()
            .and_then(phone)
            .map(|new| new.masked()),
        voter_ids,
        reported_at: signal.at.unwrap_or_else(Utc::now).to_rfc3339(),
    };
    warn!(
        account = %account.id,
        old_number = %change.masked_old_number,
        new_number = change.masked_new_number.as_deref().unwrap_or("-"),
        voters = change.voter_ids.len(),
        "{NUMBER_CHANGED_REASON}"
    );
    // Prepended in one statement on the row as it is (R5-12): signals
    // processed at the same time each add theirs, and none is lost. The
    // newest are kept; the rest of the status is the last check's,
    // untouched. An empty status starts as a whole one (`empty_status`).
    let updated = tx
        .execute(
            r#"
            UPDATE sequent_backend.messaging_account
            SET status = jsonb_set(
                    $5::jsonb || status,
                    '{details,number_changes}',
                    COALESCE((
                        SELECT jsonb_agg(change ORDER BY position)
                        FROM jsonb_array_elements(
                            jsonb_build_array($3::jsonb)
                            || COALESCE(status->'details'->'number_changes', '[]'::jsonb)
                        ) WITH ORDINALITY AS changes(change, position)
                        WHERE position <= $4
                    ), '[]'::jsonb)),
                updated_at = now()
            WHERE tenant_id = $1 AND id = $2
              -- A provider's retry of the same signal (R7-04) adds nothing.
              AND NOT COALESCE(status->'details'->'number_changes', '[]'::jsonb)
                  @> jsonb_build_array($3::jsonb)
            "#,
            &[
                &account.tenant_id,
                &account.id,
                &serde_json::to_value(&change)?,
                &(NUMBER_CHANGES_KEPT as i64),
                &empty_status()?,
            ],
        )
        .await?;
    if updated == 0 {
        return Ok(None);
    }
    Ok(Some(change))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn callbacks_go_to_the_webhook_component() {
        // The route shape itself is pinned by the relay's tests.
        let base = "https://callbacks.example.org/";
        for (provider, route) in [
            (
                MessagingProvider::WHATSAPP_CLOUD_API,
                CallbackProvider::META,
            ),
            (
                MessagingProvider::MESSENGER_SEND_API,
                CallbackProvider::META,
            ),
            (MessagingProvider::VIBER_INFOBIP, CallbackProvider::VIBER),
            (MessagingProvider::AWS_SES, CallbackProvider::AWS),
        ] {
            let readiness = callback_readiness_with(Some(base), provider, KEY);
            let expected = route.callback_url(base, KEY);
            assert!(expected.starts_with("https://callbacks.example.org/"));
            assert!(expected.ends_with(KEY));
            assert_eq!(readiness.url(), Some(expected.as_str()), "{provider}");
            assert_eq!(readiness.reason(), None);
        }
    }

    #[test]
    fn without_the_public_address_the_reason_says_so() {
        for base in [None, Some(""), Some("  ")] {
            let readiness =
                callback_readiness_with(base, MessagingProvider::MESSENGER_SEND_API, KEY);
            assert_eq!(readiness, CallbackReadiness::NotConfigured);
            assert_eq!(readiness.url(), None);
            assert_eq!(readiness.reason(), Some("Callback address not configured"));
        }
    }

    /// Regression (R1-17): an `http` component address was shown as
    /// ready, though Meta refuses such callback URLs.
    #[test]
    fn an_address_that_isnt_https_is_shown_with_its_reason() {
        let readiness = callback_readiness_with(
            Some("http://messaging-webhooks:8410"),
            MessagingProvider::WHATSAPP_CLOUD_API,
            KEY,
        );
        assert_eq!(
            readiness.url(),
            Some(format!("http://messaging-webhooks:8410/webhooks/meta/{KEY}").as_str())
        );
        assert_eq!(readiness.reason(), Some("Callback address must be https"));
        assert!(matches!(
            callback_readiness_with(
                Some("HTTPS://callbacks.example.org"),
                MessagingProvider::WHATSAPP_CLOUD_API,
                KEY
            ),
            CallbackReadiness::Ready(_)
        ));
    }

    fn stored(provider: MessagingProvider, credentials: &[CredentialName]) -> MessagingAccount {
        use sequent_core::types::messaging::{AccountSender, ProviderApproval, ReadinessPolicy};
        let sender = match provider {
            MessagingProvider::VIBER_INFOBIP => AccountSender::VIBER_INFOBIP {
                base_url: "https://a.api.infobip.com".to_string(),
                sender: "Elections".to_string(),
                approved_templates: Default::default(),
            },
            _ => AccountSender::AWS_SNS {
                sender_id: None,
                origination_number: None,
                region: None,
            },
        };
        MessagingAccount {
            id: Uuid::nil(),
            tenant_id: Uuid::nil(),
            channel: sender
                .provider()
                .channel()
                .unwrap_or(sequent_core::types::messaging::MessageChannel::SMS),
            provider,
            name: "account".to_string(),
            sender,
            credentials: credentials
                .iter()
                .map(|name| {
                    (
                        *name,
                        sequent_core::types::messaging::CredentialRecord {
                            replaced_at: "2026-10-09T00:00:00Z".to_string(),
                        },
                    )
                })
                .collect(),
            limits: Default::default(),
            provider_approval: ProviderApproval::PENDING,
            readiness: ReadinessPolicy::PROVIDER_CHECK,
            status: AccountCheck::default(),
            webhook_key: KEY.to_string(),
            is_default: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Regression (R1-05): upgraded Viber accounts have no report
    /// credentials, so every report is refused; nothing said so. One
    /// function gives the blocker the save, the UI and the log use.
    #[test]
    fn callbacks_name_what_stops_them() {
        let base = Some("https://callbacks.example.org");
        let viber =
            |credentials: &[CredentialName]| stored(MessagingProvider::VIBER_INFOBIP, credentials);
        assert_eq!(
            callback_blockers_with(base, &viber(&[CredentialName::API_KEY])),
            vec![ReadinessBlocker::REPORT_CREDENTIALS_MISSING]
        );
        assert_eq!(
            callback_blockers_with(base, &viber(&[CredentialName::REPORT_USERNAME])),
            vec![ReadinessBlocker::REPORT_CREDENTIALS_MISSING]
        );
        assert_eq!(
            callback_blockers_with(
                base,
                &viber(&[
                    CredentialName::REPORT_USERNAME,
                    CredentialName::REPORT_PASSWORD
                ])
            ),
            vec![]
        );
        assert_eq!(
            callback_blockers_with(None, &viber(&[])),
            vec![
                ReadinessBlocker::CALLBACK_NOT_CONFIGURED,
                ReadinessBlocker::REPORT_CREDENTIALS_MISSING
            ]
        );
        // SMS through SNS sends no callbacks.
        assert_eq!(
            callback_blockers_with(None, &stored(MessagingProvider::AWS_SNS, &[])),
            vec![]
        );
        assert_eq!(
            callback_blocker_reason(ReadinessBlocker::REPORT_CREDENTIALS_MISSING),
            Some("Generate the report credentials and create the Infobip subscription")
        );
        assert_eq!(
            callback_blocker_reason(ReadinessBlocker::CALLBACK_NOT_CONFIGURED),
            Some("Callback address not configured")
        );
    }

    #[test]
    fn providers_without_callbacks_need_no_address() {
        for provider in [
            MessagingProvider::AWS_SNS,
            MessagingProvider::SMTP,
            MessagingProvider::HTTP_API,
            MessagingProvider::CONSOLE,
        ] {
            for base in [None, Some("https://callbacks.example.org")] {
                let readiness = callback_readiness_with(base, provider, KEY);
                assert_eq!(readiness, CallbackReadiness::NotUsed, "{provider}");
                assert_eq!(readiness.reason(), None);
            }
        }
    }

    /// Only a credential the provider's check connects with, replacing a
    /// stored one, is checked before it is saved; typed values are refused
    /// for what they are before any provider is called.
    #[test]
    fn only_replaced_connection_credentials_are_checked_first() {
        let viber = stored(
            MessagingProvider::VIBER_INFOBIP,
            &[CredentialName::API_KEY, CredentialName::REPORT_USERNAME],
        );
        let typed = |pairs: &[(CredentialName, &str)]| -> BTreeMap<CredentialName, String> {
            pairs
                .iter()
                .map(|(name, value)| (*name, value.to_string()))
                .collect()
        };
        assert!(candidate_check_needed(
            &viber,
            &typed(&[(CredentialName::API_KEY, "new")])
        ));
        // Report credentials are Step's own: no check says anything of them.
        assert!(!candidate_check_needed(
            &viber,
            &typed(&[(CredentialName::REPORT_USERNAME, "x")])
        ));
        // A first entry has nothing to keep.
        let first = stored(MessagingProvider::VIBER_INFOBIP, &[]);
        assert!(!candidate_check_needed(
            &first,
            &typed(&[(CredentialName::API_KEY, "new")])
        ));
        assert_eq!(
            entered_credentials(&viber, &typed(&[(CredentialName::API_KEY, " new ")])).unwrap(),
            typed(&[(CredentialName::API_KEY, "new")])
        );
        assert_eq!(
            entered_credentials(&viber, &typed(&[(CredentialName::REPORT_PASSWORD, "x")]))
                .unwrap_err()
                .to_string(),
            "REPORT_PASSWORD is generated, not entered"
        );
        assert_eq!(
            entered_credentials(&viber, &typed(&[(CredentialName::API_KEY, " ")]))
                .unwrap_err()
                .to_string(),
            "API_KEY cannot be empty"
        );
    }

    /// R5-11 and R5-16: a Meta candidate replaces a stored credential only
    /// when its token is valid, unexpired and allowed to send, and Meta
    /// didn't refuse its app secret. A secret Meta couldn't check is saved,
    /// and the status shows why it isn't known.
    #[test]
    fn a_meta_candidate_must_be_able_to_send() {
        use sequent_core::types::messaging::{AppSecretStatus, TokenStatus};
        let check = |token: Option<TokenStatus>, app_secret: Option<AppSecretStatus>| {
            let mut check = AccountCheck {
                connected: true,
                checked_at: Some(Utc::now().to_rfc3339()),
                ..Default::default()
            };
            check.details.token = token;
            check.details.app_secret = app_secret;
            check
        };
        let token = |valid: bool, expires_at: Option<&str>, missing: &[&str]| TokenStatus {
            valid,
            expires_at: expires_at.map(str::to_string),
            missing_scopes: missing.iter().map(|scope| scope.to_string()).collect(),
        };
        let refused = |reason: &str| Some(format!("{reason}; {CANDIDATE_REFUSED_REASON}"));
        let whatsapp = MessagingProvider::WHATSAPP_CLOUD_API;
        let messenger = MessagingProvider::MESSENGER_SEND_API;
        assert_eq!(
            candidate_refusal(&check(Some(token(true, None, &[])), None), whatsapp),
            None
        );
        // Reading templates isn't needed to send.
        assert_eq!(
            candidate_refusal(
                &check(
                    Some(token(true, None, &["whatsapp_business_management"])),
                    None
                ),
                whatsapp
            ),
            None
        );
        assert_eq!(
            candidate_refusal(
                &check(
                    Some(token(true, None, &["whatsapp_business_messaging"])),
                    None
                ),
                whatsapp
            ),
            refused(TOKEN_CANNOT_SEND)
        );
        assert_eq!(
            candidate_refusal(
                &check(Some(token(true, None, &["pages_messaging"])), None),
                messenger
            ),
            refused(TOKEN_CANNOT_SEND)
        );
        assert_eq!(
            candidate_refusal(&check(Some(token(false, None, &[])), None), whatsapp),
            refused(TOKEN_INVALID)
        );
        assert_eq!(
            candidate_refusal(
                &check(
                    Some(token(true, Some("2020-01-01T00:00:00+00:00"), &[])),
                    None
                ),
                whatsapp
            ),
            refused(TOKEN_INVALID)
        );
        assert_eq!(
            candidate_refusal(&check(None, None), messenger),
            refused(TOKEN_NOT_CHECKED)
        );
        let secret = |valid: Option<bool>| AppSecretStatus {
            valid,
            reason: None,
        };
        assert_eq!(
            candidate_refusal(
                &check(Some(token(true, None, &[])), Some(secret(Some(false)))),
                messenger
            ),
            refused(APP_SECRET_REFUSED)
        );
        assert_eq!(
            candidate_refusal(
                &check(Some(token(true, None, &[])), Some(secret(None))),
                messenger
            ),
            None
        );
        // Other providers: connecting is enough.
        assert_eq!(
            candidate_refusal(&check(None, None), MessagingProvider::VIBER_INFOBIP),
            None
        );
        assert!(checked_by_provider(CredentialName::APP_SECRET));
    }
}
