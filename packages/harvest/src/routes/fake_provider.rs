// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Fake provider routes for development and demos: a viewer, not a mode.
//!
//! `MESSAGING_FAKE_PROVIDER_ROUTES=ENABLED` mounts these routes under
//! `/fake-provider`; that is all the variable does, and nothing else reads
//! it. Senders reach them only because the deployment's provider endpoint
//! variables (`MESSAGING_META_GRAPH_URL`, `MESSAGING_INFOBIP_URL`,
//! `MESSAGING_AWS_ENDPOINT_URL`) point here, as they would point at any
//! provider. The routes are internal, like the rest of harvest.
//!
//! - They answer the providers' send and check requests with the
//!   providers' normal success shapes. Every message is accepted.
//! - They keep the last [`CAPACITY`] messages in memory (lost on restart),
//!   each with its delivery report's result. The code a message carries is
//!   visible in that list by design, and never logged.
//! - [`REPORT_DELAY`] (3 s) after accepting a WhatsApp, Messenger or Viber
//!   message, they post a DELIVERED report to the webhook component, in the
//!   provider's own format and signature, so it takes the production path
//!   (component → `/messaging/callbacks` → verification → ledger). SMS
//!   (SNS) and email (SES) get no report, as in production without
//!   receipts.
//! - `POST /fake-provider/meta/messenger/get-started` stands in for a
//!   voter's Get Started tap on an `m.me` link.

use crate::ports::database::DatabasePools;
use crate::routes::messaging::JsonRefusal;
use crate::services::authorization::authorize;
use crate::services::dependencies::HarvestServices;
use chrono::{DateTime, Utc};
use messaging::relay::CallbackProvider;
use rocket::data::{Data, ToByteUnit};
use rocket::http::uri::Origin;
use rocket::http::{ContentType, RawStr, Status};
use rocket::request::{self, FromRequest, Request};
use rocket::serde::json::Json;
use rocket::{Build, Rocket, State};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::messaging::{
    AccountSender, CredentialName, MessageChannel, MessagingProvider,
};
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;
use windmill::postgres::messaging::{
    list_messaging_accounts, MessagingAccount,
};
use windmill::postgres::tenant::get_tenant_ids;
use windmill::services::messaging::accounts::{
    runtime_account, CALLBACK_NOT_CONFIGURED,
};
use windmill::services::messaging::keys::{tenant_key, TenantKey};
use windmill::services::vault::decrypt_with_master_secret;

/// Mounts the routes when `ENABLED`.
pub const FAKE_PROVIDER_ROUTES_ENV: &str = "MESSAGING_FAKE_PROVIDER_ROUTES";
/// Where the routes are mounted.
pub const BASE: &str = "/fake-provider";
/// How many messages the list keeps per tenant; older ones are dropped. A
/// bulk send in one tenant never pushes another tenant's messages out.
pub const CAPACITY: usize = 1000;
/// How many messages the list keeps in all, whatever their tenants.
pub const MAX_ENTRIES: usize = 20 * CAPACITY;
/// How many bytes of content the list keeps in all; the oldest messages
/// are dropped first.
pub const TOTAL_LIMIT_BYTES: usize = 64 * 1024 * 1024;
/// How long after accepting a message its report is posted. The sender
/// records the provider's message ID only once the send has answered, and
/// a report that arrives before that finds no message: three seconds leave
/// it time, also under load.
pub const REPORT_DELAY: Duration = Duration::from_secs(3);
/// The most each content field keeps, in bytes; the rest is cut and the
/// entry says `truncated`.
pub const CONTENT_LIMIT_BYTES: usize = 64 * 1024;
/// Larger request bodies are refused (emails carry their HTML).
const BODY_LIMIT_BYTES: u64 = 4 * 1024 * 1024;
/// How long the webhook component has to answer a report. Longer than the
/// component's own wait for harvest.
const REPORT_TIMEOUT: Duration = Duration::from_secs(15);

/// The templates the fake reports as approved, in every language below.
pub const FAKE_TEMPLATES: &[(&str, &str)] =
    &[("step_code", "AUTHENTICATION"), ("step_notice", "UTILITY")];
/// The languages those templates are approved in: the platform's eight,
/// and Meta's code for English.
pub const FAKE_TEMPLATE_LANGUAGES: &[&str] =
    &["en", "en_US", "es", "ca", "eu", "fr", "gl", "nl", "tl"];

/// The report's result, as the list shows it. One reason per outcome.
pub const NO_RECEIPTS_REASON: &str = "This provider sends no delivery reports";
pub const SEVERAL_ACCOUNTS_REASON: &str = "Several accounts use this sender";
pub const ACCOUNTS_UNREADABLE_REASON: &str = "The accounts couldn't be read";
pub const UNREACHABLE_REASON: &str = "The webhook component didn't answer";
/// The read of one message: it was dropped from the list, or it isn't the
/// caller's tenant's.
pub const MESSAGE_GONE_REASON: &str = "The message is no longer in the list";

/// Get Started refusals.
pub const LINK_UNREADABLE_REASON: &str =
    "Paste the m.me link from the enrollment page";
pub const LINK_WITHOUT_REF_REASON: &str =
    "The link has no ref: copy the whole link";
pub const NO_PAGE_ACCOUNT_REASON: &str =
    "No Messenger account of this organization uses this Page";
pub const SEVERAL_PAGE_ACCOUNTS_REASON: &str =
    "Several Messenger accounts of this organization use this Page";

/// Whether harvest mounts the fake provider routes. Read once, into
/// [`HarvestServices`]; it mounts the routes and does nothing else.
#[allow(non_camel_case_types)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    strum_macros::EnumString,
    strum_macros::Display,
)]
pub enum FakeProviderRoutes {
    /// Not mounted: the routes don't exist. Real deployments.
    #[default]
    DISABLED,
    /// Mounted under [`BASE`], for development and demos.
    ENABLED,
}

impl FakeProviderRoutes {
    /// Unset or blank is `DISABLED`; an unknown value is `DISABLED` too, and
    /// says so.
    pub fn from_value(value: Option<&str>) -> Self {
        let Some(value) = value.map(str::trim).filter(|v| !v.is_empty()) else {
            return Self::DISABLED;
        };
        match Self::from_str(value) {
            Ok(routes) => routes,
            Err(_) => {
                error!(
                    "{FAKE_PROVIDER_ROUTES_ENV}={value} is neither DISABLED nor \
                     ENABLED: the fake provider routes are not mounted"
                );
                Self::DISABLED
            }
        }
    }

    /// From the process environment, once, at startup.
    pub fn from_env() -> Self {
        let routes = Self::from_value(
            std::env::var(FAKE_PROVIDER_ROUTES_ENV).ok().as_deref(),
        );
        if routes == Self::ENABLED {
            warn!(
                "{FAKE_PROVIDER_ROUTES_ENV}=ENABLED: the fake provider routes \
                 are mounted at {BASE} (development and demos only)"
            );
        }
        routes
    }
}

/// Mounts the routes and their message list when `routes` is `ENABLED`.
pub fn mount(
    rocket: Rocket<Build>,
    routes: FakeProviderRoutes,
) -> Rocket<Build> {
    match routes {
        FakeProviderRoutes::DISABLED => rocket,
        FakeProviderRoutes::ENABLED => rocket
            .mount(
                BASE,
                routes![
                    meta_send,
                    meta_lookup,
                    get_started,
                    infobip_send,
                    infobip_balance,
                    infobip_logs,
                    aws_post,
                    aws_get,
                    list_messages,
                    read_message,
                ],
            )
            .manage(FakeProvider::new(REPORT_DELAY)),
    }
}

// The message list

/// What became of a message's delivery report.
#[allow(non_camel_case_types)]
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportState {
    /// SMS (SNS) and email (SES): no report, as in production.
    NO_RECEIPTS,
    /// Posted about a second after the message was accepted.
    PENDING,
    /// The component answered; `status` says what.
    ANSWERED,
    /// Not posted; `reason` says why.
    NOT_SENT,
    /// Posted, and the component didn't answer.
    UNREACHABLE,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct FakeReport {
    pub state: ReportState,
    /// The component's HTTP status, once it answered.
    pub status: Option<u16>,
    pub reason: Option<String>,
}

impl FakeReport {
    fn of(state: ReportState, reason: Option<&str>) -> Self {
        Self {
            state,
            status: None,
            reason: reason.map(str::to_string),
        }
    }
}

/// A template parameter: named (Infobip) or positional (Meta).
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct FakeParameter {
    pub name: Option<String>,
    pub value: String,
}

/// What a message said: the text, or the template with its language and
/// parameters, or the email's subject, text and HTML. `code` is the code a
/// code template carries (WhatsApp's copy-code button, Infobip's `pin`).
///
/// Each field is kept up to [`CONTENT_LIMIT_BYTES`]; `truncated` says one
/// was cut. The list leaves `html` out (`has_html` says there is one); the
/// read of one message returns it.
#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct FakeContent {
    pub text: Option<String>,
    pub subject: Option<String>,
    pub html: Option<String>,
    pub has_html: bool,
    pub truncated: bool,
    pub template_name: Option<String>,
    pub template_language: Option<String>,
    pub parameters: Vec<FakeParameter>,
    pub code: Option<String>,
}

/// `value` cut to [`CONTENT_LIMIT_BYTES`] at a character boundary.
fn capped(value: Option<String>, truncated: &mut bool) -> Option<String> {
    value.map(|mut value| {
        if value.len() > CONTENT_LIMIT_BYTES {
            let mut end = CONTENT_LIMIT_BYTES;
            while !value.is_char_boundary(end) {
                end -= 1;
            }
            value.truncate(end);
            *truncated = true;
        }
        value
    })
}

impl FakeContent {
    /// The content as the list keeps it, capped, and its HTML apart.
    fn stored(self) -> (Self, Option<String>) {
        let mut truncated = false;
        let html = capped(self.html, &mut truncated);
        let content = Self {
            text: capped(self.text, &mut truncated),
            subject: capped(self.subject, &mut truncated),
            html: None,
            has_html: html.is_some(),
            template_name: capped(self.template_name, &mut truncated),
            template_language: capped(self.template_language, &mut truncated),
            parameters: self
                .parameters
                .into_iter()
                .map(|parameter| FakeParameter {
                    name: capped(parameter.name, &mut truncated),
                    value: capped(Some(parameter.value), &mut truncated)
                        .unwrap_or_default(),
                })
                .collect(),
            code: capped(self.code, &mut truncated),
            truncated: false,
        };
        (
            Self {
                truncated,
                ..content
            },
            html,
        )
    }

    fn bytes(&self) -> usize {
        let field =
            |value: &Option<String>| value.as_ref().map_or(0, String::len);
        field(&self.text)
            + field(&self.subject)
            + field(&self.template_name)
            + field(&self.template_language)
            + field(&self.code)
            + self
                .parameters
                .iter()
                .map(|p| field(&p.name) + p.value.len())
                .sum::<usize>()
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct FakeMessage {
    /// The entry's own key in the list.
    pub id: Uuid,
    /// The message ID the fake answered the sender with. A re-sent attempt
    /// keeps its Infobip message ID, so several entries can share one.
    pub provider_message_id: String,
    pub received_at: DateTime<Utc>,
    pub channel: MessageChannel,
    pub provider: MessagingProvider,
    /// Phone number ID, Page ID, Infobip sender, SNS sender ID or From.
    pub sender: String,
    pub recipient: String,
    pub content: FakeContent,
    /// The email's HTML, capped; only the read of one message returns it.
    #[serde(skip)]
    html: Option<String>,
    /// The accounts whose sender and credential the send matched.
    #[serde(skip)]
    account_ids: Vec<Uuid>,
    /// Their tenants. The message is shown to a tenant only when it is the
    /// only one: a message several tenants could claim is shown to none.
    #[serde(skip)]
    tenant_ids: Vec<Uuid>,
}

impl FakeMessage {
    /// The tenant the message belongs to, when exactly one claims it.
    fn owner(&self) -> Option<Uuid> {
        match self.tenant_ids.as_slice() {
            [tenant] => Some(*tenant),
            _ => None,
        }
    }

    fn bytes(&self) -> usize {
        self.provider_message_id.len()
            + self.sender.len()
            + self.recipient.len()
            + self.content.bytes()
            + self.html.as_ref().map_or(0, String::len)
            + 256
    }

    /// The message with its HTML, as the read of one message returns it.
    fn with_html(&self) -> Self {
        let mut message = self.clone();
        message.content.html = self.html.clone();
        message
    }
}

/// A message and its report, as the list and the read return them.
#[derive(Debug, Clone)]
pub struct Listed {
    message: Arc<FakeMessage>,
    report: FakeReport,
}

impl Serialize for Listed {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct View<'a> {
            #[serde(flatten)]
            message: &'a FakeMessage,
            report: &'a FakeReport,
        }
        View {
            message: &self.message,
            report: &self.report,
        }
        .serialize(serializer)
    }
}

/// The routes' managed state: the message list and the HTTP client reports
/// are posted with.
#[derive(Clone)]
pub struct FakeProvider {
    inner: Arc<Inner>,
}

struct Inner {
    list: Mutex<List>,
    http: reqwest::Client,
    report_delay: Duration,
}

#[derive(Default)]
struct List {
    /// Newest first.
    entries: VecDeque<Entry>,
    bytes: usize,
}

struct Entry {
    message: Arc<FakeMessage>,
    report: FakeReport,
    bytes: usize,
}

impl FakeProvider {
    pub fn new(report_delay: Duration) -> Self {
        // Mounted once at startup; it can't fail with these settings, and
        // a client without the timeout and the redirect policy is not an
        // acceptable fallback.
        let http = reqwest::Client::builder()
            .timeout(REPORT_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("the fake provider's report client builds");
        Self {
            inner: Arc::new(Inner {
                list: Mutex::new(List::default()),
                http,
                report_delay,
            }),
        }
    }

    /// Keeps the message, newest first, within [`CAPACITY`] per tenant (the
    /// messages no single tenant claims count as one more),
    /// [`MAX_ENTRIES`] and [`TOTAL_LIMIT_BYTES`]: the oldest go first.
    fn keep(&self, message: FakeMessage, report: FakeReport) {
        let Ok(mut list) = self.inner.list.lock() else {
            return;
        };
        let owner = message.owner();
        let bytes = message.bytes();
        list.bytes += bytes;
        list.entries.push_front(Entry {
            message: Arc::new(message),
            report,
            bytes,
        });
        let over = list
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.message.owner() == owner)
            .nth(CAPACITY)
            .map(|(index, _)| index);
        if let Some(entry) = over.and_then(|index| list.entries.remove(index)) {
            list.bytes -= entry.bytes;
        }
        while list.entries.len() > MAX_ENTRIES || list.bytes > TOTAL_LIMIT_BYTES
        {
            match list.entries.pop_back() {
                Some(entry) => list.bytes -= entry.bytes,
                None => break,
            }
        }
    }

    fn set_report(&self, id: Uuid, report: FakeReport) {
        if let Ok(mut list) = self.inner.list.lock() {
            if let Some(entry) =
                list.entries.iter_mut().find(|e| e.message.id == id)
            {
                entry.report = report;
            }
        }
    }

    /// Newest first, the messages `tenant_id` alone claims. Only those are
    /// cloned under the lock, and only as shared references.
    fn visible_to(&self, tenant_id: &Uuid) -> Vec<Listed> {
        self.inner
            .list
            .lock()
            .map(|list| {
                list.entries
                    .iter()
                    .filter(|e| e.message.owner().as_ref() == Some(tenant_id))
                    .map(|e| Listed {
                        message: e.message.clone(),
                        report: e.report.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// One message `tenant_id` alone claims, with its HTML.
    fn read(&self, tenant_id: &Uuid, id: Uuid) -> Option<Listed> {
        let listed = self.inner.list.lock().ok().and_then(|list| {
            list.entries
                .iter()
                .find(|e| {
                    e.message.id == id
                        && e.message.owner().as_ref() == Some(tenant_id)
                })
                .map(|e| (e.message.clone(), e.report.clone()))
        })?;
        Some(Listed {
            message: Arc::new(listed.0.with_html()),
            report: listed.1,
        })
    }

    /// Whether the fake took `provider_message_id` from one of `accounts`.
    fn took(&self, provider_message_id: &str, accounts: &[Uuid]) -> bool {
        self.inner
            .list
            .lock()
            .map(|list| {
                list.entries.iter().any(|e| {
                    e.message.provider_message_id == provider_message_id
                        && e.message
                            .account_ids
                            .iter()
                            .any(|a| accounts.contains(a))
                })
            })
            .unwrap_or(false)
    }
}

// Accounts

/// The accounts whose sender `matches`, in `tenant` or in every tenant,
/// read the way the backend reads them. A tenant whose accounts can't be
/// read is skipped and logged.
async fn matching_accounts(
    databases: &Arc<dyn DatabasePools>,
    tenant: Option<Uuid>,
    matches: impl Fn(&AccountSender) -> bool,
) -> anyhow::Result<Vec<MessagingAccount>> {
    let pool = databases.hasura().await;
    let mut client = pool.get().await?;
    let tx = client.transaction().await?;
    let tenants: Vec<Uuid> = match tenant {
        Some(tenant) => vec![tenant],
        None => get_tenant_ids(&tx)
            .await?
            .iter()
            .filter_map(|id| Uuid::parse_str(id).ok())
            .collect(),
    };
    let mut found = vec![];
    for tenant_id in tenants {
        match list_messaging_accounts(&tx, &tenant_id).await {
            Ok(accounts) => found
                .extend(accounts.into_iter().filter(|a| matches(&a.sender))),
            Err(error) => {
                warn!(%tenant_id, "fake provider: accounts unreadable: {error:#}")
            }
        }
    }
    tx.rollback().await?;
    Ok(found)
}

/// The `Authorization` header of a provider request, as the provider reads
/// it: Meta's `Bearer` access token or Infobip's `App` API key (AWS
/// requests: [`AwsRequest`]). Never logged nor kept.
pub struct ProviderAuthorization(Option<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ProviderAuthorization {
    type Error = std::convert::Infallible;

    async fn from_request(
        request: &'r Request<'_>,
    ) -> request::Outcome<Self, Self::Error> {
        request::Outcome::Success(Self(
            request
                .headers()
                .get_one("Authorization")
                .map(str::to_string),
        ))
    }
}

impl ProviderAuthorization {
    fn scheme(&self, scheme: &str) -> Option<&str> {
        let (name, value) = self.0.as_deref()?.trim().split_once(' ')?;
        let value = value.trim();
        (name.eq_ignore_ascii_case(scheme) && !value.is_empty())
            .then_some(value)
    }

    /// Meta's access token.
    fn bearer(&self) -> Option<&str> {
        self.scheme("Bearer")
    }

    /// Infobip's API key.
    fn infobip_key(&self) -> Option<&str> {
        self.scheme("App")
    }
}

/// An AWS request as received: what its Signature Version 4 covers. A
/// send is taken only when it is signed with the secret of an account
/// whose access key ID it names (R7-19), as AWS takes it; the key ID alone
/// isn't a secret. Never logged nor kept.
pub struct AwsRequest {
    method: String,
    path: String,
    query: String,
    headers: Vec<(String, String)>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AwsRequest {
    type Error = std::convert::Infallible;

    async fn from_request(
        request: &'r Request<'_>,
    ) -> request::Outcome<Self, Self::Error> {
        request::Outcome::Success(Self {
            method: request.method().as_str().to_string(),
            path: request.uri().path().as_str().to_string(),
            query: request
                .uri()
                .query()
                .map(|query| query.as_str().to_string())
                .unwrap_or_default(),
            headers: request
                .headers()
                .iter()
                .map(|header| {
                    (
                        header.name().as_str().to_string(),
                        header.value().to_string(),
                    )
                })
                .collect(),
        })
    }
}

impl AwsRequest {
    /// The access key ID the request names, unchecked.
    fn access_key_id(&self) -> Option<&str> {
        let authorization = self
            .headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))?;
        messaging::sigv4::access_key_id(&authorization.1)
    }

    /// Whether `body` with this request was signed with `secret`.
    fn signed_with(&self, body: &[u8], secret: &str) -> bool {
        messaging::sigv4::verify(
            &messaging::sigv4::ReceivedRequest {
                method: &self.method,
                path: &self.path,
                query: &self.query,
                headers: &self.headers,
                body,
            },
            secret,
            Utc::now(),
        )
        .is_ok()
    }
}

/// Of `senders`, those `request` was signed for with their stored secret
/// access key; none is [`Refused::SIGNATURE`].
async fn signed_by(
    databases: &Arc<dyn DatabasePools>,
    senders: Senders,
    request: &AwsRequest,
    body: &[u8],
) -> Result<Senders, Refused> {
    let mut signed = vec![];
    for account in senders.0 {
        match account_credential(
            databases,
            &account,
            &[CredentialName::AWS_SECRET_ACCESS_KEY],
        )
        .await
        {
            Ok(secrets) => {
                if secrets
                    .first()
                    .and_then(Option::as_deref)
                    .is_some_and(|secret| request.signed_with(body, secret))
                {
                    signed.push(account);
                }
            }
            Err(error) => {
                error!("fake provider: credentials unreadable: {error:#}");
                return Err(Refused::ACCOUNTS_UNREADABLE);
            }
        }
    }
    Senders::new(signed).ok_or(Refused::SIGNATURE)
}

/// Why a provider request wasn't taken. Each route answers with the
/// provider's own shape.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refused {
    /// No account with this sender has the credential presented (or none
    /// was presented): the provider's 401 (AWS: 403).
    CREDENTIAL,
    /// The accounts couldn't be read: the provider's 503.
    ACCOUNTS_UNREADABLE,
    /// AWS: the request names an account's access key ID but isn't signed
    /// with its secret (R7-19): AWS's 403 `SignatureDoesNotMatch`.
    SIGNATURE,
}

/// Compares two secrets in constant time (over their digests).
fn same_secret(stored: &str, presented: &str) -> bool {
    let (stored, presented) = (
        Sha256::digest(stored.as_bytes()),
        Sha256::digest(presented.as_bytes()),
    );
    stored
        .iter()
        .zip(presented.iter())
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

/// Of the accounts whose sender `matches`, in every tenant, those whose
/// `credential` is `presented`.
async fn credentialed(
    databases: &Arc<dyn DatabasePools>,
    credential: CredentialName,
    presented: &str,
    matches: impl Fn(&AccountSender) -> bool,
) -> anyhow::Result<Vec<MessagingAccount>> {
    let candidates = matching_accounts(databases, None, matches).await?;
    if candidates.is_empty() {
        return Ok(vec![]);
    }
    let pool = databases.hasura().await;
    let mut client = pool.get().await?;
    let tx = client.transaction().await?;
    let mut found = vec![];
    for account in candidates {
        match runtime_account(&tx, &account).await {
            Ok(runtime) => {
                if runtime
                    .credential(credential)
                    .is_ok_and(|stored| same_secret(stored, presented))
                {
                    found.push(account);
                }
            }
            Err(error) => warn!(
                account = %account.id,
                "fake provider: credentials unreadable: {error:#}"
            ),
        }
    }
    tx.rollback().await?;
    Ok(found)
}

/// The accounts a provider request authenticated as. Never empty: only
/// [`authenticated`] makes one, and no account is a refusal there, so a
/// kept message always has at least one account, and more than one is
/// [`SEVERAL_ACCOUNTS_REASON`].
#[derive(Clone)]
pub(crate) struct Senders(Vec<MessagingAccount>);

impl Senders {
    /// `None` for no account.
    pub(crate) fn new(accounts: Vec<MessagingAccount>) -> Option<Senders> {
        (!accounts.is_empty()).then_some(Senders(accounts))
    }
}

/// The accounts a send is from: those whose sender `matches` and whose
/// `credential` is the one `presented`. None is a refusal, as the provider
/// refuses a credential it doesn't know.
async fn authenticated(
    databases: &Arc<dyn DatabasePools>,
    credential: CredentialName,
    presented: Option<&str>,
    matches: impl Fn(&AccountSender) -> bool,
) -> Result<Senders, Refused> {
    let Some(presented) = presented else {
        return Err(Refused::CREDENTIAL);
    };
    match credentialed(databases, credential, presented, matches).await {
        Ok(accounts) => Senders::new(accounts).ok_or(Refused::CREDENTIAL),
        Err(error) => {
            error!("fake provider: accounts unreadable: {error:#}");
            Err(Refused::ACCOUNTS_UNREADABLE)
        }
    }
}

/// The account's decrypted credentials, as the backend reads them.
async fn account_credential(
    databases: &Arc<dyn DatabasePools>,
    account: &MessagingAccount,
    names: &[CredentialName],
) -> anyhow::Result<Vec<Option<String>>> {
    let pool = databases.hasura().await;
    let mut client = pool.get().await?;
    let tx = client.transaction().await?;
    let runtime = runtime_account(&tx, account).await?;
    tx.rollback().await?;
    Ok(names
        .iter()
        .map(|name| runtime.credential(*name).ok().map(str::to_string))
        .collect())
}

// Reports

/// What a report is about.
#[derive(Debug, Clone)]
struct Subject {
    id: String,
    recipient: String,
}

/// `X-Hub-Signature-256` of `body` under the app secret.
pub fn hub_signature(app_secret: &str, body: &str) -> Option<String> {
    messaging::link::digest(app_secret.as_bytes(), body)
        .ok()
        .map(|mac| format!("sha256={mac}"))
}

/// A WhatsApp `statuses` event: the message was delivered.
pub fn whatsapp_delivered(
    business_account_id: &str,
    phone_number_id: &str,
    display_phone_number: &str,
    message_id: &str,
    recipient: &str,
    now: DateTime<Utc>,
) -> String {
    json!({
        "object": "whatsapp_business_account",
        "entry": [{
            "id": business_account_id,
            "changes": [{
                "field": "messages",
                "value": {
                    "messaging_product": "whatsapp",
                    "metadata": {
                        "display_phone_number": display_phone_number,
                        "phone_number_id": phone_number_id,
                    },
                    "statuses": [{
                        "id": message_id,
                        "status": "delivered",
                        "timestamp": now.timestamp().to_string(),
                        "recipient_id": recipient.trim_start_matches('+'),
                    }],
                },
            }],
        }],
    })
    .to_string()
}

/// A Messenger `message_deliveries` event: the message, and everything
/// before it, was delivered.
pub fn messenger_delivered(
    page_id: &str,
    page_scoped_id: &str,
    message_id: &str,
    now: DateTime<Utc>,
) -> String {
    let millis = now.timestamp_millis();
    json!({
        "object": "page",
        "entry": [{
            "id": page_id,
            "time": millis,
            "messaging": [{
                "sender": {"id": page_scoped_id},
                "recipient": {"id": page_id},
                "timestamp": millis,
                "delivery": {"mids": [message_id], "watermark": millis},
            }],
        }],
    })
    .to_string()
}

/// Meta's first-interaction postback: Get Started on an `m.me` link.
pub fn messenger_get_started(
    page_id: &str,
    page_scoped_id: &str,
    reference: &str,
    now: DateTime<Utc>,
) -> String {
    let millis = now.timestamp_millis();
    json!({
        "object": "page",
        "entry": [{
            "id": page_id,
            "time": millis,
            "messaging": [{
                "sender": {"id": page_scoped_id},
                "recipient": {"id": page_id},
                "timestamp": millis,
                "postback": {
                    "title": "Get Started",
                    "payload": "GET_STARTED",
                    "referral": {
                        "ref": reference,
                        "source": "SHORTLINK",
                        "type": "OPEN_THREAD",
                    },
                },
            }],
        }],
    })
    .to_string()
}

/// Infobip's delivery report for a Viber message.
pub fn infobip_delivered(
    message_id: &str,
    recipient: &str,
    now: DateTime<Utc>,
) -> String {
    let at = now.format("%Y-%m-%dT%H:%M:%S%.3f%z").to_string();
    json!({"results": [{
        "bulkId": null,
        "messageId": message_id,
        "to": recipient,
        "sentAt": at,
        "doneAt": at,
        "messageCount": 1,
        "price": {"pricePerMessage": 0, "currency": "EUR"},
        "status": {
            "groupId": 3,
            "groupName": "DELIVERED",
            "id": 5,
            "name": "DELIVERED_TO_HANDSET",
            "description": "Message delivered to handset",
        },
        "error": {
            "groupId": 0,
            "groupName": "OK",
            "id": 0,
            "name": "NO_ERROR",
            "description": "No Error",
            "permanent": false,
        },
        "channel": "VIBER",
    }]})
    .to_string()
}

/// `Basic` authorization of a user name and password.
fn basic(username: &str, password: &str) -> String {
    use base64::Engine;
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD
            .encode(format!("{username}:{password}"))
    )
}

/// Posts to the component; its status, or `None` when it didn't answer.
async fn post_to_component(
    http: &reqwest::Client,
    url: &str,
    headers: &[(&str, String)],
    body: String,
) -> Option<u16> {
    let mut request = http
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body);
    for (name, value) in headers {
        request = request.header(*name, value);
    }
    match request.send().await {
        Ok(response) => Some(response.status().as_u16()),
        Err(error) => {
            warn!(
                "fake provider: the webhook component didn't answer: {}",
                error.without_url()
            );
            None
        }
    }
}

/// Builds and posts the report about `subject`, with the account's own
/// secret: Meta-signed, or with the Infobip subscription's Basic auth. A
/// missing secret leaves the report unsigned, so the backend's refusal
/// shows in the list.
async fn report(
    provider: &FakeProvider,
    databases: &Arc<dyn DatabasePools>,
    component_url: &str,
    account: &MessagingAccount,
    subject: &Subject,
) -> FakeReport {
    let now = Utc::now();
    let (route, names, body) = match &account.sender {
        AccountSender::WHATSAPP_CLOUD_API {
            business_account_id,
            phone_number_id,
            display_phone_number,
            ..
        } => (
            CallbackProvider::META,
            vec![CredentialName::APP_SECRET],
            whatsapp_delivered(
                business_account_id,
                phone_number_id,
                display_phone_number,
                &subject.id,
                &subject.recipient,
                now,
            ),
        ),
        AccountSender::MESSENGER_SEND_API { page_id, .. } => (
            CallbackProvider::META,
            vec![CredentialName::APP_SECRET],
            messenger_delivered(page_id, &subject.recipient, &subject.id, now),
        ),
        AccountSender::VIBER_INFOBIP { .. } => (
            CallbackProvider::VIBER,
            vec![
                CredentialName::REPORT_USERNAME,
                CredentialName::REPORT_PASSWORD,
            ],
            infobip_delivered(&subject.id, &subject.recipient, now),
        ),
        _ => {
            return FakeReport::of(
                ReportState::NO_RECEIPTS,
                Some(NO_RECEIPTS_REASON),
            )
        }
    };
    let secrets = match account_credential(databases, account, &names).await {
        Ok(secrets) => secrets,
        Err(error) => {
            error!(account = %account.id, "fake provider: credentials unreadable: {error:#}");
            return FakeReport::of(
                ReportState::NOT_SENT,
                Some(ACCOUNTS_UNREADABLE_REASON),
            );
        }
    };
    let headers: Vec<(&str, String)> = match (route, secrets.as_slice()) {
        (CallbackProvider::META, [Some(secret)]) => {
            hub_signature(secret, &body)
                .map(|signature| ("x-hub-signature-256", signature))
                .into_iter()
                .collect()
        }
        (CallbackProvider::VIBER, [Some(username), Some(password)]) => {
            vec![("authorization", basic(username, password))]
        }
        _ => vec![],
    };
    let url = route.callback_url(component_url, &account.webhook_key);
    match post_to_component(&provider.inner.http, &url, &headers, body).await {
        Some(status) => FakeReport {
            state: ReportState::ANSWERED,
            status: Some(status),
            reason: None,
        },
        None => {
            FakeReport::of(ReportState::UNREACHABLE, Some(UNREACHABLE_REASON))
        }
    }
}

/// Keeps an accepted message, attributed to the `accounts` whose sender and
/// credential the send matched, and, for providers that report deliveries,
/// posts its report [`REPORT_DELAY`] later when exactly one account sent it.
#[allow(clippy::too_many_arguments)]
fn accept(
    provider: &FakeProvider,
    services: &HarvestServices,
    channel: MessageChannel,
    provider_kind: MessagingProvider,
    sender: String,
    subject: Subject,
    content: FakeContent,
    senders: Senders,
) {
    let Senders(accounts) = senders;
    let mut tenant_ids: Vec<Uuid> =
        accounts.iter().map(|a| a.tenant_id).collect();
    tenant_ids.sort();
    tenant_ids.dedup();
    let account_ids: Vec<Uuid> = accounts.iter().map(|a| a.id).collect();
    let receipts = matches!(
        provider_kind,
        MessagingProvider::WHATSAPP_CLOUD_API
            | MessagingProvider::MESSENGER_SEND_API
            | MessagingProvider::VIBER_INFOBIP
    );
    let component_url = services.messaging_webhooks_url.clone();
    let mut accounts = accounts;
    let (initial, target) = match (receipts, component_url) {
        (false, _) => (
            FakeReport::of(ReportState::NO_RECEIPTS, Some(NO_RECEIPTS_REASON)),
            None,
        ),
        // Never none: see [`Senders`].
        (true, _) if accounts.len() > 1 => (
            FakeReport::of(
                ReportState::NOT_SENT,
                Some(SEVERAL_ACCOUNTS_REASON),
            ),
            None,
        ),
        (true, None) => (
            FakeReport::of(
                ReportState::NOT_SENT,
                Some(CALLBACK_NOT_CONFIGURED),
            ),
            None,
        ),
        (true, Some(url)) => (
            FakeReport::of(ReportState::PENDING, None),
            accounts.pop().map(|account| (account, url)),
        ),
    };
    let id = Uuid::new_v4();
    info!(
        %channel,
        provider = %provider_kind,
        %id,
        report = ?initial.state,
        "fake provider accepted a message"
    );
    let (content, html) = content.stored();
    provider.keep(
        FakeMessage {
            id,
            provider_message_id: subject.id.clone(),
            received_at: Utc::now(),
            channel,
            provider: provider_kind,
            sender,
            recipient: subject.recipient.clone(),
            content,
            html,
            account_ids,
            tenant_ids,
        },
        initial,
    );
    if let Some((account, url)) = target {
        let provider = provider.clone();
        let databases = services.databases.clone();
        tokio::spawn(async move {
            tokio::time::sleep(provider.inner.report_delay).await;
            let result =
                report(&provider, &databases, &url, &account, &subject).await;
            info!(
                %id,
                state = ?result.state,
                status = ?result.status,
                "fake provider report done"
            );
            provider.set_report(id, result);
        });
    }
}

// Request helpers

/// A request body, up to the limit.
async fn read_body(data: Data<'_>) -> Result<Vec<u8>, Status> {
    let body = data
        .open(BODY_LIMIT_BYTES.bytes())
        .into_bytes()
        .await
        .map_err(|_| Status::BadRequest)?;
    if !body.is_complete() {
        return Err(Status::PayloadTooLarge);
    }
    Ok(body.into_inner())
}

/// Decoded `name=value` pairs of a query or form body.
fn form_pairs(raw: &str) -> Vec<(String, String)> {
    raw.split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            (
                RawStr::new(name).url_decode_lossy().into_owned(),
                RawStr::new(value).url_decode_lossy().into_owned(),
            )
        })
        .collect()
}

fn form_value<'a>(
    pairs: &'a [(String, String)],
    name: &str,
) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
}

/// A new provider-looking ID.
fn new_id(prefix: &str) -> String {
    format!("{prefix}{}", &messaging::link::new_reference()[..32])
}

fn text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_string)
}

/// A Meta template's language, parameters (named or positional) and the
/// code of a copy-code button.
fn meta_template(template: &Value) -> FakeContent {
    let mut content = FakeContent {
        template_name: text(&template["name"]),
        template_language: text(&template["language"]["code"]),
        ..Default::default()
    };
    for component in template["components"].as_array().into_iter().flatten() {
        let parameters = component["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| {
                Some(FakeParameter {
                    name: text(&p["parameter_name"]),
                    value: text(&p["text"])?,
                })
            });
        match component["type"].as_str() {
            Some("button") => {
                content.code = parameters.into_iter().next().map(|p| p.value)
            }
            _ => content.parameters.extend(parameters),
        }
    }
    content
}

/// A Meta Graph API error.
fn meta_error(status: Status, message: &str) -> (Status, Json<Value>) {
    (
        status,
        Json(json!({"error": {
            "message": message,
            "type": "OAuthException",
            "code": 100,
        }})),
    )
}

/// Meta's answer to a refused request.
fn meta_refused(refused: Refused) -> (Status, Json<Value>) {
    match refused {
        // Meta's credential is the token: there is no signature.
        Refused::CREDENTIAL | Refused::SIGNATURE => (
            Status::Unauthorized,
            Json(json!({"error": {
                "message": "Invalid OAuth access token.",
                "type": "OAuthException",
                "code": 190,
            }})),
        ),
        Refused::ACCOUNTS_UNREADABLE => (
            Status::ServiceUnavailable,
            Json(json!({"error": {
                "message": ACCOUNTS_UNREADABLE_REASON,
                "type": "OAuthException",
                "code": 2,
                "is_transient": true,
            }})),
        ),
    }
}

// WhatsApp and Messenger

/// WhatsApp Cloud API and Messenger Send API sends. A body naming
/// `messaging_product: whatsapp` is WhatsApp; anything else is Messenger.
/// Taken only with the access token of an account that uses the sender.
#[post("/meta/<_version>/<sender_id>/messages", data = "<data>")]
async fn meta_send(
    _version: &str,
    sender_id: &str,
    data: Data<'_>,
    authorization: ProviderAuthorization,
    services: &State<HarvestServices>,
    provider: &State<FakeProvider>,
) -> (Status, Json<Value>) {
    let Some(token) = authorization.bearer() else {
        return meta_refused(Refused::CREDENTIAL);
    };
    let body: Value = match read_body(data).await {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(body) => body,
            Err(_) => return meta_error(Status::BadRequest, "Invalid JSON"),
        },
        Err(status) => return meta_error(status, "Unreadable body"),
    };
    let sender = sender_id.to_string();
    if body["messaging_product"] == "whatsapp" {
        let Some(to) = text(&body["to"]) else {
            return meta_error(Status::BadRequest, "Missing recipient");
        };
        let content = match body["type"].as_str() {
            Some("template") => meta_template(&body["template"]),
            _ => FakeContent {
                text: text(&body["text"]["body"]),
                ..Default::default()
            },
        };
        let accounts = match authenticated(
            &services.databases,
            CredentialName::ACCESS_TOKEN,
            Some(token),
            |account| {
                matches!(account, AccountSender::WHATSAPP_CLOUD_API {
                    phone_number_id, ..
                } if *phone_number_id == sender)
            },
        )
        .await
        {
            Ok(accounts) => accounts,
            Err(refused) => return meta_refused(refused),
        };
        let id = new_id("wamid.FAKE");
        accept(
            provider,
            services,
            MessageChannel::WHATSAPP,
            MessagingProvider::WHATSAPP_CLOUD_API,
            sender.clone(),
            Subject {
                id: id.clone(),
                recipient: to.clone(),
            },
            content,
            accounts,
        );
        return (
            Status::Ok,
            Json(json!({
                "messaging_product": "whatsapp",
                "contacts": [{"input": to, "wa_id": to.trim_start_matches('+')}],
                "messages": [{"id": id, "message_status": "accepted"}],
            })),
        );
    }
    let Some(recipient) = text(&body["recipient"]["id"]) else {
        return meta_error(Status::BadRequest, "Missing recipient");
    };
    let message = &body["message"];
    let content = if message["template"].is_object() {
        meta_template(&message["template"])
    } else {
        FakeContent {
            text: text(&message["text"]),
            ..Default::default()
        }
    };
    let accounts = match authenticated(
        &services.databases,
        CredentialName::ACCESS_TOKEN,
        Some(token),
        |account| {
            matches!(account, AccountSender::MESSENGER_SEND_API {
                page_id, ..
            } if *page_id == sender)
        },
    )
    .await
    {
        Ok(accounts) => accounts,
        Err(refused) => return meta_refused(refused),
    };
    let id = new_id("m_FAKE");
    accept(
        provider,
        services,
        MessageChannel::MESSENGER,
        MessagingProvider::MESSENGER_SEND_API,
        sender.clone(),
        Subject {
            id: id.clone(),
            recipient: recipient.clone(),
        },
        content,
        accounts,
    );
    (
        Status::Ok,
        Json(json!({"recipient_id": recipient, "message_id": id})),
    )
}

/// The Graph API reads the account checks make, answered as connected and
/// ready:
/// - `debug_token`: valid, never expiring, with the WhatsApp and
///   `pages_messaging` scopes;
/// - the phone number: `status` CONNECTED, `name_status` APPROVED (the
///   provider approval), `account_mode` LIVE, `code_verification_status`
///   VERIFIED, `quality_rating` GREEN, `messaging_limit_tier` TIER_1K;
/// - `message_templates` of the business account or Page:
///   [`FAKE_TEMPLATES`], approved, with their category, in every language;
/// - the Page's `subscribed_apps` and its `messenger_profile` Get Started
///   button.
///
/// Every read needs an access token, as Meta's do. Any token is valid
/// (the account checks also run on credentials not stored yet), but a
/// stored account's own names and phone numbers are answered only to its
/// own token, and only when one tenant's accounts have it.
#[get("/meta/<path..>")]
async fn meta_lookup(
    path: PathBuf,
    origin: &Origin<'_>,
    authorization: ProviderAuthorization,
    services: &State<HarvestServices>,
) -> (Status, Json<Value>) {
    let Some(token) = authorization.bearer() else {
        return meta_refused(Refused::CREDENTIAL);
    };
    let segments: Vec<String> = path
        .iter()
        .filter_map(|s| s.to_str().map(str::to_string))
        .collect();
    let query = form_pairs(origin.query().map(|q| q.as_str()).unwrap_or(""));
    let Some(last) = segments.last().cloned() else {
        return meta_error(Status::BadRequest, "Unsupported get request");
    };
    let owner = segments
        .len()
        .checked_sub(2)
        .and_then(|i| segments.get(i))
        .cloned()
        .unwrap_or_default();
    match last.as_str() {
        "debug_token" => (
            Status::Ok,
            Json(json!({"data": {
                "app_id": "fake-app",
                "type": "SYSTEM_USER",
                "application": "Fake provider",
                "is_valid": true,
                "issued_at": Utc::now().timestamp(),
                "expires_at": 0,
                "data_access_expires_at": 0,
                "scopes": META_SCOPES,
                "granular_scopes": META_SCOPES
                    .iter()
                    .map(|scope| json!({"scope": scope}))
                    .collect::<Vec<_>>(),
            }})),
        ),
        "message_templates" => {
            let name = form_value(&query, "name").map(str::to_string);
            let data: Vec<Value> = fake_templates()
                .into_iter()
                .filter(|t| name.as_deref().map_or(true, |n| t["name"] == n))
                .collect();
            (Status::Ok, Json(json!({"data": data, "paging": {}})))
        }
        "subscribed_apps" => (
            Status::Ok,
            Json(json!({"data": [{
                "id": "fake-app",
                "name": "Fake provider",
                "subscribed_fields": [
                    "messages", "messaging_postbacks",
                    "message_deliveries", "messaging_referrals",
                ],
            }]})),
        ),
        "messenger_profile" => (
            Status::Ok,
            Json(json!({"data": [{
                "get_started": {"payload": "GET_STARTED"},
            }]})),
        ),
        "phone_numbers" => {
            let numbers: Vec<Value> =
                graph_accounts(services, token, |sender| {
                    matches!(sender, AccountSender::WHATSAPP_CLOUD_API {
                    business_account_id, ..
                } if *business_account_id == owner)
                })
                .await
                .iter()
                .map(|account| {
                    graph_object(&account_id_of(account), Some(account))
                })
                .collect();
            (Status::Ok, Json(json!({"data": numbers, "paging": {}})))
        }
        id => {
            let stored =
                graph_accounts(services, token, |sender| match sender {
                    AccountSender::WHATSAPP_CLOUD_API {
                        phone_number_id,
                        business_account_id,
                        ..
                    } => phone_number_id == id || business_account_id == id,
                    AccountSender::MESSENGER_SEND_API { page_id, .. } => {
                        page_id == id
                    }
                    _ => false,
                })
                .await;
            (Status::Ok, Json(graph_object(id, stored.first())))
        }
    }
}

const META_SCOPES: &[&str] = &[
    "whatsapp_business_messaging",
    "whatsapp_business_management",
    "pages_messaging",
    "pages_manage_metadata",
    "pages_show_list",
];

/// Every [`FAKE_TEMPLATES`] entry in every [`FAKE_TEMPLATE_LANGUAGES`]
/// language, approved.
pub fn fake_templates() -> Vec<Value> {
    FAKE_TEMPLATES
        .iter()
        .flat_map(|(name, category)| {
            FAKE_TEMPLATE_LANGUAGES.iter().map(move |language| {
                json!({
                    "id": format!("fake-{name}-{language}"),
                    "name": name,
                    "language": language,
                    "status": "APPROVED",
                    "category": category,
                })
            })
        })
        .collect()
}

/// The stored accounts a Graph read is about: those that `match` and have
/// `token`, when they are all one tenant's; none otherwise, or when they
/// can't be read.
async fn graph_accounts(
    services: &HarvestServices,
    token: &str,
    matches: impl Fn(&AccountSender) -> bool,
) -> Vec<MessagingAccount> {
    let accounts = credentialed(
        &services.databases,
        CredentialName::ACCESS_TOKEN,
        token,
        matches,
    )
    .await
    .unwrap_or_else(|error| {
        warn!("fake provider: accounts unreadable: {error:#}");
        vec![]
    });
    let one_tenant = accounts.first().is_some_and(|first| {
        accounts.iter().all(|a| a.tenant_id == first.tenant_id)
    });
    if one_tenant {
        accounts
    } else {
        vec![]
    }
}

fn account_id_of(account: &MessagingAccount) -> String {
    match &account.sender {
        AccountSender::WHATSAPP_CLOUD_API {
            phone_number_id, ..
        } => phone_number_id.clone(),
        AccountSender::MESSENGER_SEND_API { page_id, .. } => page_id.clone(),
        _ => String::new(),
    }
}

/// A Graph object with the fields of a ready phone number, Page and
/// business account; names come from the stored account when there is one.
fn graph_object(id: &str, account: Option<&MessagingAccount>) -> Value {
    let (display_phone_number, verified_name, page_name, username) =
        match account.map(|a| &a.sender) {
            Some(AccountSender::WHATSAPP_CLOUD_API {
                display_phone_number,
                display_name,
                ..
            }) => (
                display_phone_number.clone(),
                display_name.clone().unwrap_or_else(|| "Fake sender".into()),
                "Fake Page".to_string(),
                None,
            ),
            Some(AccountSender::MESSENGER_SEND_API {
                page_name,
                page_username,
                ..
            }) => (
                "+1 555 000 0000".to_string(),
                "Fake sender".to_string(),
                page_name.clone().unwrap_or_else(|| "Fake Page".into()),
                page_username.clone(),
            ),
            _ => (
                "+1 555 000 0000".to_string(),
                "Fake sender".to_string(),
                "Fake Page".to_string(),
                None,
            ),
        };
    json!({
        "id": id,
        // Phone number
        "display_phone_number": display_phone_number,
        "verified_name": verified_name,
        "status": "CONNECTED",
        "account_mode": "LIVE",
        "code_verification_status": "VERIFIED",
        "name_status": "APPROVED",
        "quality_rating": "GREEN",
        "messaging_limit_tier": "TIER_1K",
        "platform_type": "CLOUD_API",
        "throughput": {"level": "STANDARD"},
        "health_status": {"can_send_message": "AVAILABLE"},
        // Page
        "name": page_name,
        "username": username,
        "is_published": true,
        // Business account
        "account_review_status": "APPROVED",
        "business_verification_status": "verified",
    })
}

// Messenger Get Started

#[derive(Deserialize)]
pub struct GetStartedInput {
    link: String,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct GetStartedOutput {
    /// What the webhook component answered.
    status: Option<u16>,
    reason: Option<String>,
}

/// The Page (username or ID) and the `ref` of an `m.me` link.
pub fn parse_messenger_link(
    link: &str,
) -> Result<(String, String), &'static str> {
    let link = link.trim();
    let absolute = if link.contains("://") {
        link.to_string()
    } else {
        format!("https://{link}")
    };
    let url =
        reqwest::Url::parse(&absolute).map_err(|_| LINK_UNREADABLE_REASON)?;
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if !matches!(url.scheme(), "https" | "http")
        || !(host == "m.me" || host == "www.m.me")
    {
        return Err(LINK_UNREADABLE_REASON);
    }
    let page = url
        .path_segments()
        .and_then(|mut segments| segments.next())
        .map(|page| RawStr::new(page).percent_decode_lossy().into_owned())
        .filter(|page| !page.is_empty())
        .ok_or(LINK_UNREADABLE_REASON)?;
    let reference = url
        .query_pairs()
        .find(|(name, _)| name == "ref")
        .map(|(_, value)| value.into_owned())
        .filter(|value| !value.is_empty())
        .ok_or(LINK_WITHOUT_REF_REASON)?;
    Ok((page, reference))
}

/// A made-up Page-scoped ID, the same for every Get Started on one link.
pub fn page_scoped_id(page: &str, reference: &str) -> String {
    let digest =
        Sha256::digest(format!("{}?ref={reference}", page.to_lowercase()));
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    format!("9{:015}", u64::from_be_bytes(bytes) % 1_000_000_000_000_000)
}

/// A saved voter's sign-in uses the same conversation as verification.
/// Read only this account's live link; never expose its held code or recipient.
async fn saved_messenger_recipient(
    databases: &Arc<dyn DatabasePools>,
    account: &MessagingAccount,
    reference: &str,
) -> anyhow::Result<Option<String>> {
    let pool = databases.hasura().await;
    let mut client = pool.get().await?;
    let tx = client.transaction().await?;
    let key =
        tenant_key(&tx, &account.tenant_id, TenantKey::MessengerLink).await?;
    let digest = messaging::link::digest(&key, reference)?;
    let row = tx
        .query_opt(
            "SELECT encrypted_payload FROM sequent_backend.messenger_link
         WHERE tenant_id = $1 AND account_id = $2 AND reference_digest = $3
           AND state = 'PENDING' AND expires_at > NOW()",
            &[&account.tenant_id, &account.id, &digest],
        )
        .await?;
    let payload = row.and_then(|row| row.get::<_, Option<Vec<u8>>>(0));
    tx.commit().await?;
    let Some(payload) = payload else {
        return Ok(None);
    };
    let held: Value =
        serde_json::from_slice(&decrypt_with_master_secret(&payload).await?)?;
    Ok(held
        .get("expected_recipient")
        .and_then(Value::as_str)
        .map(str::to_string))
}

/// Stands in for a voter's Get Started tap on an `m.me` link: finds this
/// tenant's Messenger account for the link's Page, and posts Meta's signed
/// postback with the link's `ref` to the webhook component. Answers with
/// the component's status.
#[post("/meta/messenger/get-started", format = "json", data = "<body>")]
async fn get_started(
    claims: JwtClaims,
    body: Json<GetStartedInput>,
    services: &State<HarvestServices>,
    provider: &State<FakeProvider>,
) -> Result<Json<GetStartedOutput>, JsonRefusal> {
    let tenant_id = authorize_tenant(&claims)?;
    let (page, reference) = parse_messenger_link(&body.link)
        .map_err(|reason| refusal(Status::BadRequest, reason))?;
    let accounts =
        matching_accounts(&services.databases, Some(tenant_id), |sender| {
            match sender {
                AccountSender::MESSENGER_SEND_API {
                    page_id,
                    page_username,
                    ..
                } => {
                    *page_id == page
                        || page_username
                            .as_deref()
                            .is_some_and(|u| u.eq_ignore_ascii_case(&page))
                }
                _ => false,
            }
        })
        .await
        .map_err(|error| {
            error!("fake provider: accounts unreadable: {error:#}");
            refusal(Status::InternalServerError, ACCOUNTS_UNREADABLE_REASON)
        })?;
    let account = match accounts.as_slice() {
        [] => return Err(refusal(Status::NotFound, NO_PAGE_ACCOUNT_REASON)),
        [account] => account,
        _ => {
            return Err(refusal(Status::Conflict, SEVERAL_PAGE_ACCOUNTS_REASON))
        }
    };
    let AccountSender::MESSENGER_SEND_API { page_id, .. } = &account.sender
    else {
        return Err(refusal(Status::NotFound, NO_PAGE_ACCOUNT_REASON));
    };
    let Some(component_url) = services.messaging_webhooks_url.as_deref() else {
        return Err(refusal(
            Status::ServiceUnavailable,
            CALLBACK_NOT_CONFIGURED,
        ));
    };
    let secret = account_credential(
        &services.databases,
        account,
        &[CredentialName::APP_SECRET],
    )
    .await
    .map_err(|error| {
        error!(account = %account.id, "fake provider: credentials unreadable: {error:#}");
        refusal(Status::InternalServerError, ACCOUNTS_UNREADABLE_REASON)
    })?
    .pop()
    .flatten();
    let recipient = saved_messenger_recipient(&services.databases, account, &reference)
        .await
        .map_err(|error| {
            error!(account = %account.id, "fake provider: link unreadable: {error:#}");
            refusal(Status::InternalServerError, ACCOUNTS_UNREADABLE_REASON)
        })?
        .unwrap_or_else(|| page_scoped_id(&page, &reference));
    let body =
        messenger_get_started(page_id, &recipient, &reference, Utc::now());
    // Without the app secret the postback goes unsigned: the backend's
    // refusal is the answer shown.
    let headers: Vec<(&str, String)> = secret
        .and_then(|secret| hub_signature(&secret, &body))
        .map(|signature| ("x-hub-signature-256", signature))
        .into_iter()
        .collect();
    let url = CallbackProvider::META
        .callback_url(component_url, &account.webhook_key);
    let status =
        post_to_component(&provider.inner.http, &url, &headers, body).await;
    info!(account = %account.id, ?status, "fake provider posted a Get Started");
    Ok(Json(GetStartedOutput {
        status,
        reason: status.is_none().then(|| UNREACHABLE_REASON.to_string()),
    }))
}

fn refusal(status: Status, reason: &str) -> JsonRefusal {
    JsonRefusal::from((status, reason.to_string()))
}

/// The caller's tenant, once it holds `messaging-account-write` there.
fn authorize_tenant(claims: &JwtClaims) -> Result<Uuid, JsonRefusal> {
    authorize(
        claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::MESSAGING_ACCOUNT_WRITE],
    )
    .map_err(JsonRefusal::from)?;
    Uuid::parse_str(&claims.hasura_claims.tenant_id)
        .map_err(|_| refusal(Status::BadRequest, "invalid identifier"))
}

// Viber (Infobip)

/// Infobip's answer to a refused request.
fn infobip_refused(refused: Refused) -> (Status, Json<Value>) {
    let (status, id, text) = match refused {
        // Infobip's credential is the key: there is no signature.
        Refused::CREDENTIAL | Refused::SIGNATURE => (
            Status::Unauthorized,
            "UNAUTHORIZED",
            "Invalid login details",
        ),
        Refused::ACCOUNTS_UNREADABLE => (
            Status::ServiceUnavailable,
            "GENERAL_ERROR",
            ACCOUNTS_UNREADABLE_REASON,
        ),
    };
    (
        status,
        Json(json!({"requestError": {"serviceException": {
            "messageId": id, "text": text,
        }}})),
    )
}

/// Infobip Viber sends: one message per destination, with the message ID
/// the request names (Step sets it), else a new one. Taken only when every
/// message's sender is used by an account with the request's `App` key.
#[post("/infobip/viber/2/messages", data = "<data>")]
async fn infobip_send(
    data: Data<'_>,
    authorization: ProviderAuthorization,
    services: &State<HarvestServices>,
    provider: &State<FakeProvider>,
) -> (Status, Json<Value>) {
    let Some(key) = authorization.infobip_key() else {
        return infobip_refused(Refused::CREDENTIAL);
    };
    let invalid = |status: Status, text: &str| {
        (
            status,
            Json(json!({"requestError": {"serviceException": {
                "messageId": "BAD_REQUEST", "text": text,
            }}})),
        )
    };
    let body: Value = match read_body(data).await {
        Ok(bytes) => match serde_json::from_slice(&bytes) {
            Ok(body) => body,
            Err(_) => return invalid(Status::BadRequest, "Invalid JSON"),
        },
        Err(status) => return invalid(status, "Unreadable body"),
    };
    let messages: Vec<&Value> =
        body["messages"].as_array().into_iter().flatten().collect();
    // Every sender is checked before any message is taken.
    let mut senders = vec![];
    for message in &messages {
        let wanted = text(&message["sender"]).unwrap_or_default();
        match authenticated(
            &services.databases,
            CredentialName::API_KEY,
            Some(key),
            |account| {
                matches!(account, AccountSender::VIBER_INFOBIP {
                    sender, ..
                } if *sender == wanted)
            },
        )
        .await
        {
            Ok(accounts) => senders.push((wanted, accounts)),
            Err(refused) => return infobip_refused(refused),
        }
    }
    let mut answered = vec![];
    for (message, (sender, accounts)) in messages.into_iter().zip(senders) {
        let content = &message["content"];
        let mut fake = FakeContent {
            text: text(&content["text"]),
            template_name: text(&content["templateId"]),
            template_language: text(&content["language"]),
            ..Default::default()
        };
        for (name, value) in
            content["parameters"].as_object().into_iter().flatten()
        {
            let value = match value {
                Value::String(value) => value.clone(),
                other => other.to_string(),
            };
            if name == "pin" {
                fake.code = Some(value.clone());
            }
            fake.parameters.push(FakeParameter {
                name: Some(name.clone()),
                value,
            });
        }
        for destination in
            message["destinations"].as_array().into_iter().flatten()
        {
            let to = text(&destination["to"]).unwrap_or_default();
            let id = text(&destination["messageId"])
                .unwrap_or_else(|| new_id("fake-"));
            accept(
                provider,
                services,
                MessageChannel::VIBER,
                MessagingProvider::VIBER_INFOBIP,
                sender.clone(),
                Subject {
                    id: id.clone(),
                    recipient: to.clone(),
                },
                fake.clone(),
                accounts.clone(),
            );
            answered.push(json!({
                "to": to,
                "messageId": id,
                "status": {
                    "groupId": 1,
                    "groupName": "PENDING",
                    "id": 26,
                    "name": "PENDING_ACCEPTED",
                    "description": "Message sent to next instance",
                },
            }));
        }
    }
    (
        Status::Ok,
        Json(json!({"bulkId": new_id("fake-bulk-"), "messages": answered})),
    )
}

/// The Infobip account check. It needs an `App` key, as Infobip's does;
/// any key is valid (the check also runs on a key not stored yet).
#[get("/infobip/account/1/balance")]
fn infobip_balance(
    authorization: ProviderAuthorization,
) -> (Status, Json<Value>) {
    match authorization.infobip_key() {
        Some(_) => (
            Status::Ok,
            Json(json!({"balance": 1000.0, "currency": "EUR"})),
        ),
        None => infobip_refused(Refused::CREDENTIAL),
    }
}

/// Infobip's message logs, for reconciliation: a message the fake took
/// from an account with the request's `App` key is delivered; any other is
/// unknown (no results).
#[get("/infobip/viber/2/logs")]
async fn infobip_logs(
    origin: &Origin<'_>,
    authorization: ProviderAuthorization,
    services: &State<HarvestServices>,
    provider: &State<FakeProvider>,
) -> (Status, Json<Value>) {
    let Some(key) = authorization.infobip_key() else {
        return infobip_refused(Refused::CREDENTIAL);
    };
    let accounts: Vec<Uuid> = match credentialed(
        &services.databases,
        CredentialName::API_KEY,
        key,
        |sender| matches!(sender, AccountSender::VIBER_INFOBIP { .. }),
    )
    .await
    {
        Ok(accounts) => accounts.iter().map(|a| a.id).collect(),
        Err(error) => {
            error!("fake provider: accounts unreadable: {error:#}");
            return infobip_refused(Refused::ACCOUNTS_UNREADABLE);
        }
    };
    let query = form_pairs(origin.query().map(|q| q.as_str()).unwrap_or(""));
    let results: Vec<Value> = form_value(&query, "messageId")
        .filter(|id| provider.took(id, &accounts))
        .map(|id| {
            json!({
                "messageId": id,
                "status": {"groupId": 3, "groupName": "DELIVERED", "id": 5,
                    "name": "DELIVERED_TO_HANDSET"},
            })
        })
        .into_iter()
        .collect();
    (Status::Ok, Json(json!({"results": results})))
}

// SMS (SNS) and email (SES v2)

/// An SNS (query protocol, XML) answer.
fn sns_xml(status: Status, body: String) -> (Status, (ContentType, String)) {
    (status, (ContentType::XML, body))
}

fn sns_error(
    status: Status,
    code: &str,
    message: &str,
) -> (Status, (ContentType, String)) {
    sns_xml(
        status,
        format!(
            "<ErrorResponse xmlns=\"http://sns.amazonaws.com/doc/2010-03-31/\"><Error><Type>Sender</Type><Code>{code}</Code><Message>{message}</Message></Error><RequestId>{}</RequestId></ErrorResponse>",
            Uuid::new_v4()
        ),
    )
}

/// The SNS SMS attributes of a Publish, by name (`AWS.SNS.SMS.SenderID`).
fn sns_attributes(pairs: &[(String, String)]) -> BTreeMap<String, String> {
    let mut names = BTreeMap::new();
    let mut values = BTreeMap::new();
    for (name, value) in pairs {
        let Some(rest) = name.strip_prefix("MessageAttributes.entry.") else {
            continue;
        };
        let Some((index, field)) = rest.split_once('.') else {
            continue;
        };
        match field {
            "Name" => {
                names.insert(index.to_string(), value.clone());
            }
            "Value.StringValue" => {
                values.insert(index.to_string(), value.clone());
            }
            _ => {}
        }
    }
    names
        .into_iter()
        .filter_map(|(index, name)| Some((name, values.remove(&index)?)))
        .collect()
}

/// The address of `Name <address>`, or the value as it is.
fn email_address(value: &str) -> String {
    match (value.rfind('<'), value.rfind('>')) {
        (Some(start), Some(end)) if start < end => {
            value[start + 1..end].trim().to_string()
        }
        _ => value.trim().to_string(),
    }
}

const AWS_INVALID_TOKEN: &str =
    "The security token included in the request is invalid.";
/// AWS's own answer to a request not signed with the key's secret.
const AWS_SIGNATURE_MISMATCH: &str = "The request signature we calculated does not match the signature you provided.";

/// SNS's answer to a refused request.
fn sns_refused(refused: Refused) -> (Status, (ContentType, String)) {
    match refused {
        Refused::CREDENTIAL => sns_error(
            Status::Forbidden,
            "InvalidClientTokenId",
            AWS_INVALID_TOKEN,
        ),
        Refused::ACCOUNTS_UNREADABLE => sns_error(
            Status::ServiceUnavailable,
            "ServiceUnavailable",
            ACCOUNTS_UNREADABLE_REASON,
        ),
        Refused::SIGNATURE => sns_error(
            Status::Forbidden,
            "SignatureDoesNotMatch",
            AWS_SIGNATURE_MISMATCH,
        ),
    }
}

/// SES v2's answer to a refused request.
fn ses_refused(refused: Refused) -> (Status, (ContentType, String)) {
    let (status, message) = match refused {
        Refused::CREDENTIAL => (Status::Forbidden, AWS_INVALID_TOKEN),
        Refused::SIGNATURE => (Status::Forbidden, AWS_SIGNATURE_MISMATCH),
        Refused::ACCOUNTS_UNREADABLE => {
            (Status::ServiceUnavailable, ACCOUNTS_UNREADABLE_REASON)
        }
    };
    (
        status,
        (ContentType::JSON, json!({"message": message}).to_string()),
    )
}

/// SNS (`Action=Publish`, `Action=GetSMSSandboxAccountStatus`) at the
/// endpoint's root, and SES v2 `SendEmail`. Every request needs a SigV4
/// `Authorization`; a send is taken only when it names the access key ID
/// of an account that uses the sender and is signed with that account's
/// secret access key ([`AwsRequest`]).
#[post("/aws/<path..>", data = "<data>")]
async fn aws_post(
    path: PathBuf,
    data: Data<'_>,
    aws: AwsRequest,
    services: &State<HarvestServices>,
    provider: &State<FakeProvider>,
) -> (Status, (ContentType, String)) {
    let segments: Vec<&str> = path.iter().filter_map(|s| s.to_str()).collect();
    let Some(key_id) = aws.access_key_id() else {
        return match segments.as_slice() {
            [] => sns_refused(Refused::CREDENTIAL),
            _ => ses_refused(Refused::CREDENTIAL),
        };
    };
    let body = match read_body(data).await {
        Ok(body) => body,
        Err(status) => return (status, (ContentType::Plain, String::new())),
    };
    match segments.as_slice() {
        [] => sns(&body, key_id, &aws, services, provider).await,
        ["v2", "email", "outbound-emails"] => {
            ses_send(&body, key_id, &aws, services, provider).await
        }
        _ => (
            Status::NotFound,
            (
                ContentType::JSON,
                json!({"message": "Unknown operation"}).to_string(),
            ),
        ),
    }
}

async fn sns(
    body: &[u8],
    key_id: &str,
    aws: &AwsRequest,
    services: &HarvestServices,
    provider: &FakeProvider,
) -> (Status, (ContentType, String)) {
    let pairs = form_pairs(&String::from_utf8_lossy(body));
    let request_id = Uuid::new_v4();
    match form_value(&pairs, "Action") {
        Some("GetSMSSandboxAccountStatus") => sns_xml(
            Status::Ok,
            format!(
                "<GetSMSSandboxAccountStatusResponse xmlns=\"http://sns.amazonaws.com/doc/2010-03-31/\"><GetSMSSandboxAccountStatusResult><IsInSandbox>false</IsInSandbox></GetSMSSandboxAccountStatusResult><ResponseMetadata><RequestId>{request_id}</RequestId></ResponseMetadata></GetSMSSandboxAccountStatusResponse>"
            ),
        ),
        Some("Publish") => {
            let Some(phone) = form_value(&pairs, "PhoneNumber") else {
                return sns_error(
                    Status::BadRequest,
                    "InvalidParameter",
                    "Only SMS to a PhoneNumber is supported",
                );
            };
            let attributes = sns_attributes(&pairs);
            let sender_id = attributes.get("AWS.SNS.SMS.SenderID").cloned();
            let origination =
                attributes.get("AWS.MM.SMS.OriginationNumber").cloned();
            let accounts = match authenticated(
                &services.databases,
                CredentialName::AWS_ACCESS_KEY_ID,
                Some(key_id),
                |account| match account {
                    AccountSender::AWS_SNS {
                        sender_id: account_sender,
                        origination_number,
                        ..
                    } => {
                        (sender_id.is_some() && *account_sender == sender_id)
                            || (origination.is_some()
                                && *origination_number == origination)
                    }
                    _ => false,
                },
            )
            .await
            {
                Ok(senders) => senders,
                Err(refused) => return sns_refused(refused),
            };
            let accounts =
                match signed_by(&services.databases, accounts, aws, body).await
                {
                    Ok(senders) => senders,
                    Err(refused) => return sns_refused(refused),
                };
            let id = Uuid::new_v4().to_string();
            accept(
                provider,
                services,
                MessageChannel::SMS,
                MessagingProvider::AWS_SNS,
                sender_id.clone().or(origination.clone()).unwrap_or_default(),
                Subject {
                    id: id.clone(),
                    recipient: phone.to_string(),
                },
                FakeContent {
                    text: form_value(&pairs, "Message").map(str::to_string),
                    ..Default::default()
                },
                accounts,
            );
            sns_xml(
                Status::Ok,
                format!(
                    "<PublishResponse xmlns=\"http://sns.amazonaws.com/doc/2010-03-31/\"><PublishResult><MessageId>{id}</MessageId></PublishResult><ResponseMetadata><RequestId>{request_id}</RequestId></ResponseMetadata></PublishResponse>"
                ),
            )
        }
        _ => sns_error(Status::BadRequest, "InvalidAction", "Unsupported action"),
    }
}

async fn ses_send(
    raw: &[u8],
    key_id: &str,
    aws: &AwsRequest,
    services: &HarvestServices,
    provider: &FakeProvider,
) -> (Status, (ContentType, String)) {
    let Ok(body) = serde_json::from_slice::<Value>(raw) else {
        return (
            Status::BadRequest,
            (
                ContentType::JSON,
                json!({"message": "Invalid JSON"}).to_string(),
            ),
        );
    };
    let from = text(&body["FromEmailAddress"]).unwrap_or_default();
    let address = email_address(&from);
    let simple = &body["Content"]["Simple"];
    let accounts = match authenticated(
        &services.databases,
        CredentialName::AWS_ACCESS_KEY_ID,
        Some(key_id),
        |account| {
            matches!(account, AccountSender::AWS_SES { from_address, .. }
                if from_address.eq_ignore_ascii_case(&address))
        },
    )
    .await
    {
        Ok(senders) => senders,
        Err(refused) => return ses_refused(refused),
    };
    let accounts =
        match signed_by(&services.databases, accounts, aws, raw).await {
            Ok(senders) => senders,
            Err(refused) => return ses_refused(refused),
        };
    let id = new_id("fake-ses-");
    let recipients: Vec<String> = body["Destination"]["ToAddresses"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(text)
        .collect();
    accept(
        provider,
        services,
        MessageChannel::EMAIL,
        MessagingProvider::AWS_SES,
        from.clone(),
        Subject {
            id: id.clone(),
            recipient: recipients.join(", "),
        },
        FakeContent {
            subject: text(&simple["Subject"]["Data"]),
            text: text(&simple["Body"]["Text"]["Data"]),
            html: text(&simple["Body"]["Html"]["Data"]),
            ..Default::default()
        },
        accounts,
    );
    (
        Status::Ok,
        (ContentType::JSON, json!({"MessageId": id}).to_string()),
    )
}

/// The SES v2 account check, answered as sending and in production. It
/// needs a SigV4 `Authorization`; any key is valid (the check also runs on
/// keys not stored yet).
#[get("/aws/<path..>")]
fn aws_get(path: PathBuf, aws: AwsRequest) -> (Status, (ContentType, String)) {
    if aws.access_key_id().is_none() {
        return ses_refused(Refused::CREDENTIAL);
    }
    let segments: Vec<&str> = path.iter().filter_map(|s| s.to_str()).collect();
    match segments.as_slice() {
        ["v2", "email", "account"] => (
            Status::Ok,
            (
                ContentType::JSON,
                json!({
                    "DedicatedIpAutoWarmupEnabled": false,
                    "EnforcementStatus": "HEALTHY",
                    "ProductionAccessEnabled": true,
                    "SendingEnabled": true,
                    "SendQuota": {
                        "Max24HourSend": 50000.0,
                        "MaxSendRate": 14.0,
                        "SentLast24Hours": 0.0,
                    },
                })
                .to_string(),
            ),
        ),
        _ => (
            Status::NotFound,
            (
                ContentType::JSON,
                json!({"message": "Unknown operation"}).to_string(),
            ),
        ),
    }
}

// The viewer's list

#[derive(Serialize, Debug)]
pub struct MessagesOutput {
    messages: Vec<Listed>,
}

/// The messages the fake took, newest first, as the caller's tenant may see
/// them (the Hasura action `messaging_fake_provider_messages`). Codes are
/// visible here by design; email HTML is not (`has_html` says there is
/// one): [`read_message`] returns it.
///
/// A message belongs to the tenant of the account whose sender and
/// credential the send matched. A message several tenants' accounts match
/// (the same sender and credential) is shown to none of them.
#[post("/messages")]
fn list_messages(
    claims: JwtClaims,
    provider: &State<FakeProvider>,
) -> Result<Json<MessagesOutput>, JsonRefusal> {
    let tenant_id = authorize_tenant(&claims)?;
    Ok(Json(MessagesOutput {
        messages: provider.visible_to(&tenant_id),
    }))
}

#[derive(Deserialize)]
pub struct ReadMessageInput {
    /// The entry's `id` in the list.
    id: String,
}

/// One message of the caller's tenant, with its email HTML (the Hasura
/// action `messaging_fake_provider_message`).
#[post("/message", data = "<body>")]
fn read_message(
    claims: JwtClaims,
    body: Json<ReadMessageInput>,
    provider: &State<FakeProvider>,
) -> Result<Json<Listed>, JsonRefusal> {
    let tenant_id = authorize_tenant(&claims)?;
    Uuid::parse_str(body.id.trim())
        .ok()
        .and_then(|id| provider.read(&tenant_id, id))
        .map(Json)
        .ok_or_else(|| refusal(Status::NotFound, MESSAGE_GONE_REASON))
}

#[cfg(test)]
#[path = "../../tests/support/fake_provider_routes.rs"]
mod route_tests;
