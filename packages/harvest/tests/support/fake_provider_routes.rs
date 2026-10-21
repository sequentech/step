// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The fake provider routes. The reports they build are checked with the
//! backend's own parsers and signature checks. End to end, harvest is
//! served on a loopback port with the routes mounted, in front of the
//! migrated test database, with the webhook component in front of harvest,
//! as on the development stack: the provider adapters send to the routes
//! through the deployment's endpoint variables, and the reports come back
//! through the component and `/messaging/callbacks` to the ledger.

use super::*;
use crate::route_services::rows::{self, Event};
use crate::route_services::service_tokens::RealmSigner;
use crate::route_services::{Services, RELAY_SECRET};
use crate::test_claims::Claims;
use messaging::providers::{
    AWS_ENDPOINT_URL_ENV, INFOBIP_URL_ENV, META_GRAPH_URL_ENV,
};
use messaging::webhooks::{self as parsed, WebhookEvent};
use sequent_core::types::messaging::{
    callback_blocker, purpose_readiness, EventChannelConfig,
    EventMessagingConfig, MessageAttemptState, MessageChannel, MessagePurpose,
    OutOfWindowPolicy, ProviderApproval, ReadinessBlocker, TemplateBinding,
    EVENT_MESSAGING_CONFIG_VERSION,
};
use std::time::Instant;
use windmill::services::messaging::config::account_summary;

const CHILD: &str = "HARVEST_FAKE_PROVIDER_CHILD";
const USER_ID: &str = "3f2b8c1e-6d4a-4c1b-9e7f-2a5d8b0c9e1a";
const PHONE: &str = "+34600000001";
const MASTER_SECRET: &str =
    "0a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20212223242526272829";
const APP_SECRET: &str = "synthetic-app-secret-of-a-meta-application";

#[test]
fn the_switch_is_an_enum_and_only_enabled_mounts() {
    use FakeProviderRoutes::*;
    for (value, expected) in [
        (None, DISABLED),
        (Some(""), DISABLED),
        (Some("  "), DISABLED),
        (Some("DISABLED"), DISABLED),
        (Some("ENABLED"), ENABLED),
        (Some(" ENABLED "), ENABLED),
        // Unknown values don't mount, and say so.
        (Some("enabled"), DISABLED),
        (Some("true"), DISABLED),
    ] {
        assert_eq!(
            FakeProviderRoutes::from_value(value),
            expected,
            "{value:?}"
        );
    }
    assert_eq!(FakeProviderRoutes::default(), DISABLED);
}

#[test]
fn an_m_me_link_gives_its_page_and_ref() {
    for link in [
        "https://m.me/elections?ref=abc123",
        "  m.me/elections?ref=abc123 ",
        "http://www.m.me/elections?ref=abc123&other=1",
    ] {
        assert_eq!(
            parse_messenger_link(link),
            Ok(("elections".to_string(), "abc123".to_string())),
            "{link}"
        );
    }
    assert_eq!(
        parse_messenger_link("https://m.me/123456?ref=a%2Bb"),
        Ok(("123456".to_string(), "a+b".to_string()))
    );
    for (link, reason) in [
        (
            "https://example.org/elections?ref=abc",
            LINK_UNREADABLE_REASON,
        ),
        ("https://m.me/?ref=abc", LINK_UNREADABLE_REASON),
        ("not a link at all", LINK_UNREADABLE_REASON),
        ("ftp://m.me/elections?ref=abc", LINK_UNREADABLE_REASON),
        ("https://m.me/elections", LINK_WITHOUT_REF_REASON),
        ("https://m.me/elections?ref=", LINK_WITHOUT_REF_REASON),
    ] {
        assert_eq!(parse_messenger_link(link), Err(reason), "{link}");
    }
}

#[test]
fn a_links_page_scoped_id_is_made_up_digits_and_stable() {
    let id = page_scoped_id("elections", "abc");
    assert_eq!(id.len(), 16);
    assert!(id.chars().all(|c| c.is_ascii_digit()));
    assert_eq!(page_scoped_id("Elections", "abc"), id);
    assert_ne!(page_scoped_id("elections", "abd"), id);
    assert!(messaging::destination::Destination::parse(
        MessageChannel::MESSENGER,
        &id
    )
    .is_ok());
}

fn whatsapp_sender() -> AccountSender {
    AccountSender::WHATSAPP_CLOUD_API {
        business_account_id: "waba-1".to_string(),
        phone_number_id: "pn-1".to_string(),
        display_phone_number: "+1 555 000 0001".to_string(),
        display_name: None,
        api_version: "v23.0".to_string(),
        api_base_url: None,
    }
}

fn messenger_sender() -> AccountSender {
    AccountSender::MESSENGER_SEND_API {
        page_id: "page-1".to_string(),
        page_name: None,
        page_username: Some("elections".to_string()),
        api_version: "v23.0".to_string(),
        api_base_url: None,
    }
}

#[test]
fn reports_verify_and_parse_as_the_backend_reads_them() {
    let now = Utc::now();
    let body = whatsapp_delivered(
        "waba-1",
        "pn-1",
        "+1 555 000 0001",
        "wamid.1",
        PHONE,
        now,
    );
    let signature = hub_signature(APP_SECRET, &body).unwrap();
    assert!(parsed::meta::verify_signature(
        APP_SECRET,
        Some(&signature),
        body.as_bytes()
    ));
    assert!(!parsed::meta::verify_signature(
        "another-secret",
        Some(&signature),
        body.as_bytes()
    ));
    let events =
        parsed::meta::parse_events(&whatsapp_sender(), body.as_bytes())
            .unwrap();
    assert!(matches!(
        events.as_slice(),
        [WebhookEvent::Status(parsed::StatusReport {
            provider_message_id, state: MessageAttemptState::DELIVERED, ..
        })] if provider_message_id == "wamid.1"
    ));

    let body = messenger_delivered("page-1", "9123", "m_1", now);
    let events =
        parsed::meta::parse_events(&messenger_sender(), body.as_bytes())
            .unwrap();
    assert!(events.iter().any(|event| matches!(event,
        WebhookEvent::Status(parsed::StatusReport {
            provider_message_id, state: MessageAttemptState::DELIVERED, ..
        }) if provider_message_id == "m_1")));

    let body = messenger_get_started("page-1", "9123", "ref-1", now);
    let events =
        parsed::meta::parse_events(&messenger_sender(), body.as_bytes())
            .unwrap();
    assert!(matches!(
        events.as_slice(),
        [WebhookEvent::MessengerReferral(parsed::MessengerReferral {
            page_scoped_id, reference, ..
        })] if page_scoped_id == "9123" && reference == "ref-1"
    ));

    let body = infobip_delivered("keycloak:k:1", "34600000001", now);
    let events = parsed::infobip::parse_events(body.as_bytes()).unwrap();
    assert!(matches!(
        events.as_slice(),
        [WebhookEvent::Status(parsed::StatusReport {
            provider_message_id,
            state: MessageAttemptState::DELIVERED,
            error_code: None,
            ..
        })] if provider_message_id == "keycloak:k:1"
    ));
}

#[test]
fn templates_are_approved_in_every_language() {
    let templates = fake_templates();
    assert_eq!(
        templates.len(),
        FAKE_TEMPLATES.len() * FAKE_TEMPLATE_LANGUAGES.len()
    );
    assert!(templates.iter().all(|t| t["status"] == "APPROVED"));
    assert!(
        templates
            .iter()
            .any(|t| t["name"] == "step_code"
                && t["category"] == "AUTHENTICATION")
    );
}

#[test]
fn sms_senders_and_email_addresses_are_read_from_the_requests() {
    let pairs = form_pairs(
        "Action=Publish&PhoneNumber=%2B34600000001&Message=Hello+there\
         &MessageAttributes.entry.1.Name=AWS.SNS.SMS.SMSType\
         &MessageAttributes.entry.1.Value.StringValue=Transactional\
         &MessageAttributes.entry.2.Name=AWS.SNS.SMS.SenderID\
         &MessageAttributes.entry.2.Value.DataType=String\
         &MessageAttributes.entry.2.Value.StringValue=Elections",
    );
    assert_eq!(form_value(&pairs, "PhoneNumber"), Some(PHONE));
    assert_eq!(form_value(&pairs, "Message"), Some("Hello there"));
    let attributes = sns_attributes(&pairs);
    assert_eq!(
        attributes.get("AWS.SNS.SMS.SenderID").map(String::as_str),
        Some("Elections")
    );
    assert_eq!(attributes.len(), 2);
    assert_eq!(
        email_address("Elections <noreply@example.org>"),
        "noreply@example.org"
    );
    assert_eq!(
        email_address(" noreply@example.org "),
        "noreply@example.org"
    );
}

#[test]
fn meta_template_parameters_keep_their_names() {
    let content = meta_template(&json!({
        "name": "step_notice",
        "language": {"code": "en"},
        "components": [{"type": "body", "parameters": [
            {"type": "text", "parameter_name": "post", "text": "Central office"},
            {"type": "text", "text": "positional"},
        ]}],
    }));
    assert_eq!(
        content.parameters,
        vec![
            FakeParameter {
                name: Some("post".to_string()),
                value: "Central office".to_string(),
            },
            FakeParameter {
                name: None,
                value: "positional".to_string(),
            },
        ]
    );
}

fn kept(provider_message_id: &str, tenant_ids: Vec<Uuid>) -> FakeMessage {
    FakeMessage {
        id: Uuid::new_v4(),
        provider_message_id: provider_message_id.to_string(),
        received_at: Utc::now(),
        channel: MessageChannel::SMS,
        provider: MessagingProvider::AWS_SNS,
        sender: "Elections".to_string(),
        recipient: PHONE.to_string(),
        content: FakeContent::default(),
        html: None,
        account_ids: vec![],
        tenant_ids,
    }
}

fn no_receipts() -> FakeReport {
    FakeReport::of(ReportState::NO_RECEIPTS, Some(NO_RECEIPTS_REASON))
}

#[test]
fn the_list_keeps_the_last_messages_newest_first_per_tenant() {
    let provider = FakeProvider::new(REPORT_DELAY);
    let (mine, theirs) = (Uuid::new_v4(), Uuid::new_v4());
    provider.keep(kept("theirs", vec![theirs]), no_receipts());
    for n in 0..CAPACITY + 5 {
        provider.keep(kept(&format!("m-{n}"), vec![mine]), no_receipts());
    }
    provider.keep(kept("both", vec![mine, theirs]), no_receipts());
    let visible = provider.visible_to(&mine);
    assert_eq!(visible.len(), CAPACITY);
    assert_eq!(
        visible[0].message.provider_message_id,
        format!("m-{}", CAPACITY + 4)
    );
    assert_eq!(visible[CAPACITY - 1].message.provider_message_id, "m-5");
    // Another tenant's bulk send doesn't push this one's messages out, and
    // a message two tenants claim is shown to neither.
    let theirs_visible = provider.visible_to(&theirs);
    assert_eq!(theirs_visible.len(), 1);
    assert_eq!(theirs_visible[0].message.provider_message_id, "theirs");
    assert!(visible
        .iter()
        .all(|m| m.message.provider_message_id != "both"));
    // Serialized for the viewer, without the accounts and tenants.
    let value = serde_json::to_value(&visible[0]).unwrap();
    assert_eq!(value["id"], visible[0].message.id.to_string());
    assert_eq!(value["report"]["state"], "NO_RECEIPTS");
    assert!(value.get("tenant_ids").is_none());
    assert!(value.get("account_ids").is_none());
    assert!(value.get("html").is_none());
}

#[test]
fn a_report_goes_to_its_own_entry_when_message_ids_repeat() {
    let provider = FakeProvider::new(REPORT_DELAY);
    let tenant = Uuid::new_v4();
    let (first, second) =
        (kept("same", vec![tenant]), kept("same", vec![tenant]));
    let (first_id, second_id) = (first.id, second.id);
    let pending = FakeReport::of(ReportState::PENDING, None);
    provider.keep(first, pending.clone());
    provider.keep(second, pending);
    for id in [first_id, second_id] {
        provider.set_report(
            id,
            FakeReport {
                state: ReportState::ANSWERED,
                status: Some(200),
                reason: None,
            },
        );
    }
    let listed = provider.visible_to(&tenant);
    assert_eq!(listed.len(), 2);
    assert_ne!(listed[0].message.id, listed[1].message.id);
    assert!(listed
        .iter()
        .all(|m| m.report.state == ReportState::ANSWERED));
}

#[test]
fn content_is_capped_and_the_html_is_read_one_message_at_a_time() {
    let html = "é".repeat(CONTENT_LIMIT_BYTES); // two bytes each
    let (content, stored) = FakeContent {
        subject: Some("Notice".to_string()),
        html: Some(html),
        ..Default::default()
    }
    .stored();
    let stored = stored.unwrap();
    assert!(stored.len() <= CONTENT_LIMIT_BYTES && stored.len() > 0);
    assert!(content.truncated && content.has_html && content.html.is_none());
    let short = FakeContent {
        text: Some("Your code is 000000".to_string()),
        ..Default::default()
    }
    .stored();
    assert!(!short.0.truncated && !short.0.has_html && short.1.is_none());

    let provider = FakeProvider::new(REPORT_DELAY);
    let (mine, theirs) = (Uuid::new_v4(), Uuid::new_v4());
    let mut message = kept("email", vec![mine]);
    message.content = content;
    message.html = Some(stored.clone());
    let id = message.id;
    provider.keep(message, no_receipts());
    let listed = serde_json::to_value(&provider.visible_to(&mine)[0]).unwrap();
    assert_eq!(listed["content"]["html"], Value::Null);
    assert_eq!(listed["content"]["has_html"], true);
    let read = serde_json::to_value(provider.read(&mine, id).unwrap()).unwrap();
    assert_eq!(read["content"]["html"], stored);
    assert_eq!(read["content"]["truncated"], true);
    assert!(provider.read(&theirs, id).is_none());
    assert!(provider.read(&mine, Uuid::new_v4()).is_none());
}

#[test]
fn credentials_are_read_from_the_authorization_header() {
    let header = |value: &str| ProviderAuthorization(Some(value.to_string()));
    assert_eq!(header("Bearer token-1").bearer(), Some("token-1"));
    assert_eq!(header("bearer token-1").bearer(), Some("token-1"));
    assert_eq!(header("Bearer ").bearer(), None);
    assert_eq!(header("App key-1").bearer(), None);
    assert_eq!(header("App key-1").infobip_key(), Some("key-1"));
    assert_eq!(
        messaging::sigv4::access_key_id(
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20261010/eu-west-1/sns/\
             aws4_request, SignedHeaders=host;x-amz-date, Signature=00"
        ),
        Some("AKIDEXAMPLE")
    );
    assert_eq!(messaging::sigv4::access_key_id("Bearer AKIDEXAMPLE"), None);
    // Zero accounts never reach a kept message: no account is a refusal.
    assert!(Senders::new(vec![]).is_none());
    assert_eq!(ProviderAuthorization(None).bearer(), None);
    assert!(same_secret("token-1", "token-1"));
    assert!(!same_secret("token-1", "token-2"));
    assert!(!same_secret("token-1", ""));
}

// End to end

/// Runs `test` alone in a fresh process: the dispatcher reads the
/// provider endpoint variables once per process, and the child gets a
/// database of its own.
fn isolated(test: &str) -> bool {
    if std::env::var(CHILD).as_deref() == Ok(test) {
        return true;
    }
    let log = tempfile::NamedTempFile::new().unwrap();
    let output = log.reopen().unwrap();
    let mut child =
        std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env(CHILD, test)
            .env("SUPER_ADMIN_TENANT_ID", "fixture-super-admin")
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .spawn()
            .unwrap();
    let deadline = Instant::now() + Duration::from_secs(240);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!(
                "fake provider test timed out: {}",
                std::fs::read_to_string(log.path()).unwrap()
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(
        status.success(),
        "{}",
        std::fs::read_to_string(log.path()).unwrap()
    );
    false
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Serves `rocket` on a loopback port until the process ends.
async fn serve(rocket: Rocket<Build>, port: u16) {
    let config = rocket::Config {
        address: std::net::Ipv4Addr::LOCALHOST.into(),
        port,
        log_level: rocket::config::LogLevel::Off,
        cli_colors: false,
        shutdown: rocket::config::Shutdown {
            ctrlc: false,
            #[cfg(unix)]
            signals: Default::default(),
            ..Default::default()
        },
        ..rocket::Config::debug_default()
    };
    let ignited = rocket.configure(config).ignite().await.expect("ignites");
    tokio::spawn(ignited.launch());
    let deadline = Instant::now() + Duration::from_secs(20);
    while tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .is_err()
    {
        assert!(Instant::now() < deadline, "port {port} never opened");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Harvest with the fake provider routes and the webhook component, served
/// as on the development stack.
struct Stack {
    http: reqwest::Client,
    harvest: String,
    services: Services,
    event: Event,
}

impl Stack {
    async fn start() -> Self {
        std::env::set_var("MASTER_SECRET", MASTER_SECRET);
        let (harvest_port, component_port) = (free_port(), free_port());
        let harvest = format!("http://127.0.0.1:{harvest_port}");
        let component = format!("http://127.0.0.1:{component_port}");
        // What the development .env sets, on loopback ports.
        std::env::set_var(META_GRAPH_URL_ENV, format!("{harvest}{BASE}/meta"));
        std::env::set_var(INFOBIP_URL_ENV, format!("{harvest}{BASE}/infobip"));
        std::env::set_var(AWS_ENDPOINT_URL_ENV, format!("{harvest}{BASE}/aws"));
        std::env::set_var(
            messaging::relay::WEBHOOKS_PUBLIC_URL_ENV,
            &component,
        );
        let services = Services::on_test_database()
            .await
            .with_fake_provider_routes()
            .with_messaging_webhooks_url(&component);
        let event = rows::event(&services.hasura).await;
        serve(
            crate::build_application_with(
                services.harvest_services(),
                crate::test_claims::signing::issuers(),
            ),
            harvest_port,
        )
        .await;
        serve(
            messaging_webhooks::build(messaging_webhooks::Config {
                callbacks_url: format!(
                    "{harvest}{}",
                    messaging::relay::CALLBACKS_PATH
                ),
                relay_secret: RELAY_SECRET.to_vec(),
                max_body_bytes: messaging::relay::DEFAULT_MAX_BODY_BYTES,
                timeout: Duration::from_secs(10),
                max_in_flight: messaging::relay::DEFAULT_MAX_IN_FLIGHT,
            })
            .expect("the component builds"),
            component_port,
        )
        .await;
        Self {
            http: reqwest::Client::new(),
            harvest,
            services,
            event,
        }
    }

    fn admin(&self) -> Claims {
        Claims::new(&self.event.tenant_id, USER_ID).roles([
            Permissions::MESSAGING_ACCOUNT_WRITE,
            Permissions::MESSAGING_CONFIG_WRITE,
        ])
    }

    async fn post_as(
        &self,
        path: &str,
        authorization: &str,
        body: Value,
    ) -> (u16, Value) {
        let response = self
            .http
            .post(format!("{}{path}", self.harvest))
            .header("Authorization", authorization)
            .json(&body)
            .send()
            .await
            .expect("harvest answers");
        let status = response.status().as_u16();
        let text = response.text().await.unwrap_or_default();
        (
            status,
            serde_json::from_str(&text).unwrap_or(Value::String(text)),
        )
    }

    async fn post(&self, path: &str, body: Value) -> (u16, Value) {
        self.post_as(path, &self.admin().bearer(), body).await
    }

    /// A stored account with its credentials, checked: its id.
    async fn account(
        &self,
        sender: Value,
        credentials: Value,
        generate: Value,
    ) -> String {
        self.account_as(&self.admin(), sender, credentials, generate)
            .await
    }

    /// [`Stack::account`] by the administrator `claims` names.
    async fn account_as(
        &self,
        claims: &Claims,
        sender: Value,
        credentials: Value,
        generate: Value,
    ) -> String {
        let bearer = claims.bearer();
        let post = |path: &'static str, body: Value| {
            let bearer = bearer.clone();
            async move { self.post_as(path, &bearer, body).await }
        };
        let (status, created) = post(
            "/messaging/accounts/upsert",
            json!({"name": sender["provider"], "sender": sender}),
        )
        .await;
        assert_eq!(status, 200, "{created}");
        let id = created["id"].as_str().unwrap().to_string();
        let mut body = json!({"id": id, "credentials": credentials});
        for (name, value) in generate.as_object().into_iter().flatten() {
            body[name] = value.clone();
        }
        let (status, replaced) =
            post("/messaging/accounts/credentials", body).await;
        assert_eq!(status, 200, "{replaced}");
        let (status, checked) =
            post("/messaging/accounts/check", json!({"id": id})).await;
        assert_eq!(status, 200, "{checked}");
        assert_eq!(checked["status"]["connected"], true, "{checked}");
        assert_eq!(checked["status"]["production_access"], true, "{checked}");
        id
    }

    /// A test message through the account: the ledger row's id.
    async fn test_send(&self, id: &str, body: Value) -> Uuid {
        let mut body = body;
        body["id"] = json!(id);
        let (status, sent) = self.post("/messaging/accounts/test", body).await;
        assert_eq!(status, 200, "{sent}");
        assert_eq!(sent["state"], "ACCEPTED", "{sent}");
        Uuid::parse_str(sent["message_id"].as_str().unwrap()).unwrap()
    }

    /// A provider request straight to the fake routes, as a sender (or
    /// anyone who reaches harvest) would make it.
    async fn raw(
        &self,
        method: reqwest::Method,
        path: &str,
        authorization: Option<&str>,
        content_type: &str,
        body: String,
    ) -> (u16, String) {
        let mut request = self
            .http
            .request(method, format!("{}{path}", self.harvest))
            .header("Content-Type", content_type)
            .body(body);
        if let Some(value) = authorization {
            request = request.header("Authorization", value);
        }
        let response = request.send().await.expect("harvest answers");
        let status = response.status().as_u16();
        (status, response.text().await.unwrap_or_default())
    }

    /// An AWS request to the fake routes signed with `secret` (Signature
    /// Version 4), as the SDK signs it.
    async fn aws_signed(
        &self,
        path: &str,
        access_key_id: &str,
        secret: &str,
        service: &str,
        content_type: &str,
        body: String,
    ) -> (u16, String) {
        let host = self.harvest.trim_start_matches("http://").to_string();
        let mut headers = vec![
            ("host".to_string(), host),
            ("content-type".to_string(), content_type.to_string()),
            (
                "x-amz-date".to_string(),
                messaging::sigv4::amz_date(chrono::Utc::now()),
            ),
        ];
        let authorization = messaging::sigv4::authorization(
            &messaging::sigv4::ReceivedRequest {
                method: "POST",
                path,
                query: "",
                headers: &headers,
                body: body.as_bytes(),
            },
            access_key_id,
            secret,
            "eu-west-1",
            service,
        )
        .expect("signed");
        headers.push(("authorization".to_string(), authorization));
        let mut request =
            self.http.post(format!("{}{path}", self.harvest)).body(body);
        for (name, value) in headers.iter().filter(|(name, _)| name != "host") {
            request = request.header(name.as_str(), value.as_str());
        }
        let response = request.send().await.expect("harvest answers");
        let status = response.status().as_u16();
        (status, response.text().await.unwrap_or_default())
    }

    /// The list as the administrator `claims` names sees it.
    async fn list_as(&self, claims: &Claims) -> Vec<Value> {
        let (status, list) = self
            .post_as(&format!("{BASE}/messages"), &claims.bearer(), json!({}))
            .await;
        assert_eq!(status, 200, "{list}");
        list["messages"].as_array().cloned().unwrap_or_default()
    }

    async fn provider_id(&self, message_id: &Uuid) -> String {
        rows::query(
            &self.services.hasura,
            "SELECT provider_message_id FROM sequent_backend.message
             WHERE id = $1",
            &[message_id],
        )
        .await[0]
            .get(0)
    }

    async fn messages(&self) -> Vec<Value> {
        let (status, list) =
            self.post(&format!("{BASE}/messages"), json!({})).await;
        assert_eq!(status, 200, "{list}");
        list["messages"].as_array().cloned().unwrap_or_default()
    }

    /// The listed message the ledger row was sent as.
    async fn listed(&self, message_id: &Uuid) -> Value {
        let provider_id: String = rows::query(
            &self.services.hasura,
            "SELECT provider_message_id FROM sequent_backend.message
             WHERE id = $1",
            &[message_id],
        )
        .await[0]
            .get(0);
        self.messages()
            .await
            .into_iter()
            .find(|m| is_provider_message(m, &provider_id))
            .unwrap_or_else(|| panic!("{provider_id} is listed"))
    }

    /// The listed message once its report is no longer pending.
    async fn reported(&self, id: &str) -> Value {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let message = self
                .messages()
                .await
                .into_iter()
                .find(|m| m["id"] == id)
                .expect("still listed");
            if message["report"]["state"] != "PENDING" {
                return message;
            }
            assert!(Instant::now() < deadline, "report pending: {message}");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn ledger_state(&self, message_id: &Uuid) -> String {
        rows::query(
            &self.services.hasura,
            "SELECT state FROM sequent_backend.message WHERE id = $1",
            &[message_id],
        )
        .await[0]
            .get(0)
    }

    /// Sent, reported through the component and DELIVERED in the ledger.
    async fn assert_delivered(&self, message_id: &Uuid) -> Value {
        let listed = self.listed(message_id).await;
        let reported = self.reported(listed["id"].as_str().unwrap()).await;
        assert_eq!(
            reported["report"],
            json!({"state": "ANSWERED", "status": 200, "reason": null}),
            "{reported}"
        );
        assert_eq!(self.ledger_state(message_id).await, "DELIVERED");
        reported
    }
}

/// The accounts' stored checks: ready for codes and notices in English
/// (the readiness the Settings > Messaging status shows), accepted by the
/// event configuration's own validation with every channel on, and carrying
/// what the fake reported as a connected, ready provider.
async fn assert_ready(stack: &Stack, accounts: &[(&str, MessageChannel)]) {
    let tenant_id = Uuid::parse_str(&stack.event.tenant_id).unwrap();
    let mut client = stack.services.hasura.get().await.expect("database");
    let tx = client.transaction().await.expect("transaction");
    let stored = list_messaging_accounts(&tx, &tenant_id)
        .await
        .expect("accounts readable");
    tx.rollback().await.ok();
    let mut summaries = vec![];
    let mut channels = vec![];
    for (id, channel) in accounts {
        let account = stored
            .iter()
            .find(|account| account.id.to_string() == *id)
            .unwrap_or_else(|| panic!("account {id} is stored"));
        assert_eq!(account.channel, *channel);
        let summary = account_summary(account);
        assert!(
            summary.callback_blockers.is_empty(),
            "{channel}: {summary:?}"
        );
        let check = &summary.check;
        assert!(
            check.connected && check.production_access,
            "{channel}: {check:?}"
        );
        assert_eq!(check.reason, None, "{channel}");
        for purpose in [MessagePurpose::OTP, MessagePurpose::NOTICE] {
            let mut readiness = purpose_readiness(
                &summary.capabilities,
                summary.provider_approval,
                summary.identity.as_ref(),
                check,
                purpose,
                Some("en"),
            );
            readiness.blockers.extend(callback_blocker(
                *channel,
                purpose,
                !summary
                    .callback_blockers
                    .contains(&ReadinessBlocker::CALLBACK_NOT_CONFIGURED),
            ));
            assert!(readiness.is_ready(), "{channel} {purpose}: {readiness:?}");
        }
        let details = &check.details;
        match channel {
            MessageChannel::WHATSAPP | MessageChannel::MESSENGER => {
                let token = details.token.as_ref().expect("debug_token read");
                assert!(
                    token.valid && token.missing_scopes.is_empty(),
                    "{token:?}"
                );
                assert_eq!(details.templates_reason, None, "{channel}");
                assert!(!details.templates.is_empty(), "{channel}");
                assert!(details.templates.iter().all(|template| {
                    template.status == "APPROVED" && template.category.is_some()
                }));
            }
            _ => assert_eq!(details.token, None, "{channel}"),
        }
        match channel {
            MessageChannel::WHATSAPP => {
                let number = details.whatsapp_number.as_ref().expect("number");
                assert_eq!(number.status.as_deref(), Some("CONNECTED"));
                assert_eq!(number.name_status.as_deref(), Some("APPROVED"));
                assert_eq!(number.quality_rating.as_deref(), Some("GREEN"));
                assert_eq!(number.messaging_limit.as_deref(), Some("TIER_1K"));
                assert_eq!(
                    details.provider_approval,
                    Some(ProviderApproval::CONFIRMED)
                );
            }
            MessageChannel::MESSENGER => {
                let page = details.messenger.as_ref().expect("Page status");
                assert_eq!(page.pages_messaging, Some(true));
                assert!(page.subscribed);
                assert_eq!(page.get_started, Some(true));
            }
            MessageChannel::VIBER => {
                let login =
                    details.partner_login.as_ref().expect("partner login");
                assert!(login.accepted, "{login:?}");
                assert_eq!(login.reason, None);
            }
            _ => {}
        }
        let template = |purpose: MessagePurpose, name: &str| TemplateBinding {
            purpose,
            key: None,
            language: "en".to_string(),
            provider_template: name.to_string(),
            provider_language: None,
        };
        let templates = match channel {
            MessageChannel::WHATSAPP => vec![
                template(MessagePurpose::OTP, "step_code"),
                template(MessagePurpose::NOTICE, "step_notice"),
            ],
            MessageChannel::VIBER => vec![
                template(MessagePurpose::OTP, "fake-viber-code"),
                template(MessagePurpose::NOTICE, "fake-viber-notice"),
            ],
            // Notices outside the 24-hour window go as the utility template.
            MessageChannel::MESSENGER => {
                vec![template(MessagePurpose::NOTICE, "step_notice")]
            }
            _ => vec![],
        };
        channels.push(EventChannelConfig {
            channel: *channel,
            account_id: id.to_string(),
            purposes: vec![MessagePurpose::OTP, MessagePurpose::NOTICE],
            templates,
            out_of_window: OutOfWindowPolicy::DISABLED,
        });
        summaries.push(summary);
    }
    let config = EventMessagingConfig {
        version: EVENT_MESSAGING_CONFIG_VERSION,
        channels,
        election_channels: Default::default(),
    };
    assert_eq!(
        config.validate(&stack.event.tenant_id, &summaries, &[]),
        Ok(()),
        "every channel is ready"
    );
}

/// Whether the listed entry is the message the fake answered `id` for.
fn is_provider_message(message: &Value, id: &str) -> bool {
    message["provider_message_id"] == id || message["id"] == id
}

/// Records what a regression check found wrong, so one run shows all.
macro_rules! expect {
    ($failures:expr, $ok:expr, $($what:tt)+) => {
        if !$ok {
            $failures.push(format!($($what)+));
        }
    };
}

fn tenant_admin(tenant_id: &str) -> Claims {
    Claims::new(tenant_id, USER_ID).roles([
        Permissions::MESSAGING_ACCOUNT_WRITE,
        Permissions::MESSAGING_CONFIG_WRITE,
    ])
}

/// An AWS Signature Version 4 `Authorization` header naming `access_key_id`.
fn sigv4(access_key_id: &str, service: &str) -> String {
    format!(
        "AWS4-HMAC-SHA256 Credential={access_key_id}/20261010/eu-west-1/\
         {service}/aws4_request, SignedHeaders=host;x-amz-date, Signature=00"
    )
}

/// The regressions of review R6: a send needs its account's own credential
/// (R6-01), and belongs to that account's tenant only (R6-02); the list
/// carries no email HTML, which one entry's read returns, capped (R6-03);
/// a re-sent message ID makes two entries, each with its report (R6-04);
/// the report leaves the sender time to record the message (R6-12).
#[rocket::async_test]
async fn sends_need_the_account_credential_and_belong_to_its_tenant() {
    if !isolated("routes::fake_provider::route_tests::sends_need_the_account_credential_and_belong_to_its_tenant") {
        return;
    }
    let stack = Stack::start().await;
    let mut failures: Vec<String> = vec![];
    let a = stack.admin();
    let b = tenant_admin(&rows::event(&stack.services.hasura).await.tenant_id);
    let c = tenant_admin(&rows::event(&stack.services.hasura).await.tenant_id);
    let post = reqwest::Method::POST;
    let get = reqwest::Method::GET;

    // R6-01: no send without the account's own credential.
    let phone_number_id = unique("pn");
    let whatsapp = |name: &str| {
        json!({
            "provider": "WHATSAPP_CLOUD_API",
            "business_account_id": unique("waba"),
            "phone_number_id": phone_number_id,
            "display_phone_number": "+1 555 000 0001",
            "display_name": name,
            "api_version": "v23.0",
        })
    };
    let meta =
        |token: &str| json!({"ACCESS_TOKEN": token, "APP_SECRET": APP_SECRET});
    let verify = json!({"generate_verify_token": true});
    let whatsapp_a = stack
        .account_as(&a, whatsapp("Tenant A"), meta("token-a"), verify.clone())
        .await;
    let send_path = format!("{BASE}/meta/v23.0/{phone_number_id}/messages");
    let text_to = |to: &str| {
        json!({"messaging_product": "whatsapp", "to": to, "type": "text",
               "text": {"body": "Your code is 111111"}})
        .to_string()
    };
    for authorization in [None, Some("Bearer not-the-token")] {
        let (status, _) = stack
            .raw(
                post.clone(),
                &send_path,
                authorization,
                "application/json",
                text_to("+34600000099"),
            )
            .await;
        expect!(
            failures,
            status == 401,
            "R6-01: a WhatsApp send with {authorization:?} answered {status}"
        );
    }
    let viber_sender = unique("Viber");
    stack
        .account_as(
            &a,
            json!({"provider": "VIBER_INFOBIP",
                   "base_url": "https://infobip.example.org",
                   "sender": viber_sender,
                   "approved_templates": {"OTP": {"en": "fake-viber-code"}}}),
            json!({"API_KEY": "key-a"}),
            json!({"generate_report_credentials": true}),
        )
        .await;
    let viber_send = |message_id: &str, to: &str| {
        json!({"messages": [{"sender": viber_sender,
               "destinations": [{"to": to, "messageId": message_id}],
               "content": {"text": "Your code is 111111"}}]})
        .to_string()
    };
    for authorization in [None, Some("App not-the-key")] {
        let (status, _) = stack
            .raw(
                post.clone(),
                &format!("{BASE}/infobip/viber/2/messages"),
                authorization,
                "application/json",
                viber_send(&unique("forged-"), "+34600000098"),
            )
            .await;
        expect!(
            failures,
            status == 401,
            "R6-01: a Viber send with {authorization:?} answered {status}"
        );
    }
    let sms_sender = unique("Sms");
    stack
        .account_as(
            &a,
            json!({"provider": "AWS_SNS", "sender_id": sms_sender,
                   "region": "eu-west-1"}),
            json!({"AWS_ACCESS_KEY_ID": "AKIDTENANTA00000001",
                   "AWS_SECRET_ACCESS_KEY": "synthetic-secret"}),
            json!({}),
        )
        .await;
    let publish = format!(
        "Action=Publish&PhoneNumber=%2B34600000097&Message=x\
         &MessageAttributes.entry.1.Name=AWS.SNS.SMS.SenderID\
         &MessageAttributes.entry.1.Value.DataType=String\
         &MessageAttributes.entry.1.Value.StringValue={sms_sender}"
    );
    for authorization in [None, Some(sigv4("AKIDSOMEONEELSE00001", "sns"))] {
        let (status, _) = stack
            .raw(
                post.clone(),
                &format!("{BASE}/aws/"),
                authorization.as_deref(),
                "application/x-www-form-urlencoded",
                publish.clone(),
            )
            .await;
        expect!(
            failures,
            status == 403,
            "R6-01: an SMS send with {authorization:?} answered {status}"
        );
    }
    let refused = stack.list_as(&a).await;
    for to in ["+34600000099", "+34600000098", "+34600000097"] {
        expect!(
            failures,
            refused.iter().all(|m| m["recipient"] != to),
            "R6-01: the refused send to {to} is listed"
        );
    }

    // R6-02: tenant B reuses tenant A's number with its own token.
    stack
        .account_as(&b, whatsapp("Tenant B"), meta("token-b"), verify.clone())
        .await;
    let sent = stack
        .test_send(
            &whatsapp_a,
            json!({"purpose": "OTP", "destination": PHONE, "language": "en",
                   "template": "step_code"}),
        )
        .await;
    let answered_at = Instant::now();
    let provider_id = stack.provider_id(&sent).await;
    let entry = stack.listed(&sent).await;
    let reported = stack.reported(entry["id"].as_str().unwrap()).await;
    let reported_after = answered_at.elapsed();
    expect!(
        failures,
        reported["report"]["state"] == "ANSWERED",
        "R6-02: tenant A's report: {}",
        reported["report"]
    );
    expect!(
        failures,
        stack.ledger_state(&sent).await == "DELIVERED",
        "R6-02: tenant A's message is not DELIVERED"
    );
    // R6-12: the sender has time to record the message's ID.
    expect!(
        failures,
        reported_after >= Duration::from_millis(2500),
        "R6-12: the report came {reported_after:?} after the send's answer"
    );
    expect!(
        failures,
        stack
            .list_as(&b)
            .await
            .iter()
            .all(|m| !is_provider_message(m, &provider_id)),
        "R6-02: tenant B lists tenant A's message"
    );
    for (token, name) in [("token-a", "Tenant A"), ("token-b", "Tenant B")] {
        let (status, body) = stack
            .raw(
                get.clone(),
                &format!(
                    "{BASE}/meta/v23.0/{phone_number_id}?fields=verified_name"
                ),
                Some(&format!("Bearer {token}")),
                "application/json",
                String::new(),
            )
            .await;
        let body: Value = serde_json::from_str(&body).unwrap_or_default();
        expect!(
            failures,
            status == 200 && body["verified_name"] == name,
            "R6-02: the check with {token} answered {status} {body}"
        );
    }
    // Tenant C's account has tenant A's number and token: the provider
    // can't tell them apart, so neither tenant sees the message.
    stack
        .account_as(&c, whatsapp("Tenant C"), meta("token-a"), verify.clone())
        .await;
    let (status, _) = stack
        .raw(
            post.clone(),
            &send_path,
            Some("Bearer token-a"),
            "application/json",
            text_to("+34600000077"),
        )
        .await;
    expect!(
        failures,
        status == 200,
        "R6-02: the shared-token send answered {status}"
    );
    for (tenant, claims) in [("A", &a), ("B", &b), ("C", &c)] {
        expect!(failures,
            stack.list_as(claims).await.iter().all(|m| m["recipient"] != "+34600000077"),
            "R6-02: tenant {tenant} lists a message several tenants could claim");
    }

    // R6-03: the list carries no HTML; one entry's read does, capped.
    let from = format!("{}@example.org", unique("noreply-"));
    stack
        .account_as(
            &a,
            json!({"provider": "AWS_SES", "from_address": from,
                   "from_name": "Elections", "region": "eu-west-1"}),
            json!({"AWS_ACCESS_KEY_ID": "AKIDTENANTA00000002",
                   "AWS_SECRET_ACCESS_KEY": "synthetic-secret"}),
            json!({}),
        )
        .await;
    let html = format!("<p>{}</p>", "x".repeat(300 * 1024));
    let email = json!({"FromEmailAddress": from,
                    "Destination": {"ToAddresses": ["voter@example.org"]},
                    "Content": {"Simple": {"Subject": {"Data": "Big"},
                        "Body": {"Text": {"Data": "text"},
                                 "Html": {"Data": html}}}}})
    .to_string();
    // R7-19: the access key ID isn't a secret. A send naming the account's
    // key ID but not signed with its secret is refused, as AWS refuses it.
    let ses_path = format!("{BASE}/aws/v2/email/outbound-emails");
    for (what, answered) in [
        (
            "an unsigned send",
            stack
                .raw(
                    post.clone(),
                    &ses_path,
                    Some(&sigv4("AKIDTENANTA00000002", "ses")),
                    "application/json",
                    email.clone(),
                )
                .await,
        ),
        (
            "a send signed with another secret",
            stack
                .aws_signed(
                    &ses_path,
                    "AKIDTENANTA00000002",
                    "not-the-secret",
                    "ses",
                    "application/json",
                    email.clone(),
                )
                .await,
        ),
    ] {
        expect!(
            failures,
            answered.0 == 403,
            "R7-19: {what} with the account's key ID answered {}",
            answered.0
        );
    }
    let (status, answer) = stack
        .aws_signed(
            &ses_path,
            "AKIDTENANTA00000002",
            "synthetic-secret",
            "ses",
            "application/json",
            email,
        )
        .await;
    expect!(
        failures,
        status == 200,
        "R6-03: the email send answered {status}"
    );
    let email_id = serde_json::from_str::<Value>(&answer).unwrap_or_default()
        ["MessageId"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    match stack
        .list_as(&a)
        .await
        .into_iter()
        .find(|m| is_provider_message(m, &email_id))
    {
        None => failures.push("R6-03: the email is not listed".to_string()),
        Some(listed) => {
            expect!(
                failures,
                listed["content"]["html"].is_null(),
                "R6-03: the list carries the email's HTML"
            );
            expect!(
                failures,
                listed["content"]["has_html"] == true,
                "R6-03: the list doesn't say the email has HTML"
            );
            let (status, full) = stack
                .post(&format!("{BASE}/message"), json!({"id": listed["id"]}))
                .await;
            let stored = full["content"]["html"].as_str().map(str::len);
            expect!(failures,
                status == 200 && stored.is_some_and(|n| n <= CONTENT_LIMIT_BYTES)
                    && full["content"]["truncated"] == true,
                "R6-03: the entry's read answered {status} with {stored:?} bytes \
                 of HTML, truncated {}", full["content"]["truncated"]);
            let (status, _) = stack
                .post_as(
                    &format!("{BASE}/message"),
                    &b.bearer(),
                    json!({"id": listed["id"]}),
                )
                .await;
            expect!(
                failures,
                status == 404,
                "R6-03: tenant B read tenant A's email: {status}"
            );
        }
    }

    // R6-04: the same message ID twice gives two entries, each reported.
    let attempt = unique("attempt-");
    for _ in 0..2 {
        let (status, _) = stack
            .raw(
                post.clone(),
                &format!("{BASE}/infobip/viber/2/messages"),
                Some("App key-a"),
                "application/json",
                viber_send(&attempt, PHONE),
            )
            .await;
        expect!(
            failures,
            status == 200,
            "R6-04: the Viber send answered {status}"
        );
    }
    let deadline = Instant::now() + Duration::from_secs(30);
    let entries = loop {
        let entries: Vec<Value> = stack
            .list_as(&a)
            .await
            .into_iter()
            .filter(|m| is_provider_message(m, &attempt))
            .collect();
        if entries.iter().all(|m| m["report"]["state"] != "PENDING")
            || Instant::now() >= deadline
        {
            break entries;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    };
    let ids: std::collections::BTreeSet<String> =
        entries.iter().map(|m| m["id"].to_string()).collect();
    expect!(
        failures,
        entries.len() == 2 && ids.len() == 2,
        "R6-04: {} entries with ids {ids:?}",
        entries.len()
    );
    expect!(
        failures,
        entries.iter().all(|m| m["report"]["state"] == "ANSWERED"),
        "R6-04: reports {:?}",
        entries
            .iter()
            .map(|m| m["report"].clone())
            .collect::<Vec<_>>()
    );

    assert!(
        failures.is_empty(),
        "R6 regressions:\n{}",
        failures.join("\n")
    );
}

/// A report the backend can't verify is refused: the fake posts it unsigned
/// (or without the report Basic auth) when the account lacks the secret,
/// the component relays harvest's refusal, the list shows the report as
/// answered with that status, and the ledger row stays ACCEPTED.
#[rocket::async_test]
async fn a_report_without_the_account_secret_is_refused_and_the_ledger_stays() {
    if !isolated("routes::fake_provider::route_tests::a_report_without_the_account_secret_is_refused_and_the_ledger_stays") {
        return;
    }
    let stack = Stack::start().await;
    let phone_number_id = unique("pn");
    // WhatsApp without the app secret: the statuses report goes unsigned.
    let whatsapp = stack
        .account(
            json!({
                "provider": "WHATSAPP_CLOUD_API",
                "business_account_id": unique("waba"),
                "phone_number_id": phone_number_id,
                "display_phone_number": "+1 555 000 0001",
                "api_version": "v23.0",
            }),
            json!({"ACCESS_TOKEN": "token-unsigned"}),
            json!({"generate_verify_token": true}),
        )
        .await;
    // Viber without the report credentials: the report has no Basic auth.
    let viber = stack
        .account(
            json!({
                "provider": "VIBER_INFOBIP",
                "base_url": "https://infobip.example.org",
                "sender": unique("Viber"),
                "approved_templates": {"OTP": {"en": "fake-viber-code"}},
            }),
            json!({"API_KEY": "key-unsigned"}),
            json!({}),
        )
        .await;
    let mut failures: Vec<String> = vec![];
    for (account, body) in [
        (
            &whatsapp,
            json!({"purpose": "OTP", "destination": PHONE,
                           "language": "en", "template": "step_code"}),
        ),
        (
            &viber,
            json!({"purpose": "OTP", "destination": PHONE,
                        "language": "en"}),
        ),
    ] {
        let sent = stack.test_send(account, body).await;
        let listed = stack.listed(&sent).await;
        let reported = stack.reported(listed["id"].as_str().unwrap()).await;
        let status = reported["report"]["status"].as_u64();
        expect!(
            failures,
            reported["report"]["state"] == "ANSWERED"
                && status.is_some_and(|s| (400..500).contains(&s)),
            "{}: the unverifiable report was {}",
            listed["channel"],
            reported["report"]
        );
        let state = stack.ledger_state(&sent).await;
        expect!(
            failures,
            state == "ACCEPTED",
            "{}: the ledger moved to {state}",
            listed["channel"]
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn unique(prefix: &str) -> String {
    format!("{prefix}{}", &Uuid::new_v4().simple().to_string()[..12])
}

#[rocket::async_test]
async fn adapters_send_and_check_through_the_routes_and_reports_reach_the_ledger(
) {
    if !isolated("routes::fake_provider::route_tests::adapters_send_and_check_through_the_routes_and_reports_reach_the_ledger") {
        return;
    }
    let stack = Stack::start().await;

    // WhatsApp: the code template, signed statuses report.
    let phone_number_id = unique("pn");
    let whatsapp = stack
        .account(
            json!({
                "provider": "WHATSAPP_CLOUD_API",
                "business_account_id": unique("waba"),
                "phone_number_id": phone_number_id,
                "display_phone_number": "+1 555 000 0001",
                "api_version": "v23.0",
            }),
            json!({"ACCESS_TOKEN": "synthetic-token", "APP_SECRET": APP_SECRET}),
            json!({"generate_verify_token": true}),
        )
        .await;
    let sent = stack
        .test_send(
            &whatsapp,
            json!({"purpose": "OTP", "destination": PHONE, "language": "en",
                   "template": "step_code"}),
        )
        .await;
    let listed = stack.assert_delivered(&sent).await;
    assert_eq!(listed["channel"], "WHATSAPP");
    assert_eq!(listed["provider"], "WHATSAPP_CLOUD_API");
    assert_eq!(listed["sender"], phone_number_id.as_str());
    assert_eq!(listed["recipient"], PHONE);
    assert_eq!(listed["content"]["template_name"], "step_code");
    assert_eq!(listed["content"]["template_language"], "en");
    assert_eq!(listed["content"]["code"], "000000");

    // Viber: Infobip's report with the subscription's Basic auth.
    let viber_sender = unique("Fake");
    let viber = stack
        .account(
            json!({
                "provider": "VIBER_INFOBIP",
                "base_url": "https://infobip.example.org",
                "sender": viber_sender,
                "approved_templates": {"OTP": {"en": "fake-viber-code"},
                                       "NOTICE": {"en": "fake-viber-notice"}},
            }),
            json!({"API_KEY": "synthetic-key"}),
            json!({"generate_report_credentials": true}),
        )
        .await;
    let sent = stack
        .test_send(
            &viber,
            json!({"purpose": "OTP", "destination": PHONE, "language": "en"}),
        )
        .await;
    let listed = stack.assert_delivered(&sent).await;
    assert_eq!(listed["channel"], "VIBER");
    assert_eq!(listed["sender"], viber_sender.as_str());
    assert_eq!(listed["content"]["template_name"], "fake-viber-code");
    assert_eq!(listed["content"]["code"], "000000");

    // SMS through SNS: no report, as in production.
    let sms_sender = unique("Sms");
    let sms = stack
        .account(
            json!({"provider": "AWS_SNS", "sender_id": sms_sender,
                   "region": "eu-west-1"}),
            json!({"AWS_ACCESS_KEY_ID": "AKIDEXAMPLE",
                   "AWS_SECRET_ACCESS_KEY": "synthetic-secret"}),
            json!({}),
        )
        .await;
    let sent = stack
        .test_send(&sms, json!({"purpose": "NOTICE", "destination": PHONE}))
        .await;
    let listed = stack.listed(&sent).await;
    assert_eq!(listed["channel"], "SMS");
    assert_eq!(listed["sender"], sms_sender.as_str());
    assert_eq!(listed["recipient"], PHONE);
    assert_eq!(
        listed["content"]["text"],
        "This is a test message from Step."
    );
    assert_eq!(
        listed["report"],
        json!({"state": "NO_RECEIPTS", "status": null, "reason": NO_RECEIPTS_REASON})
    );
    assert_eq!(stack.ledger_state(&sent).await, "ACCEPTED");

    // Email through SES v2: no report either.
    let from = format!("{}@example.org", unique("noreply-"));
    let email = stack
        .account(
            json!({"provider": "AWS_SES", "from_address": from,
                   "from_name": "Elections", "region": "eu-west-1"}),
            json!({"AWS_ACCESS_KEY_ID": "AKIDEXAMPLE",
                   "AWS_SECRET_ACCESS_KEY": "synthetic-secret"}),
            json!({}),
        )
        .await;
    let sent = stack
        .test_send(
            &email,
            json!({"purpose": "NOTICE", "destination": "voter@example.org"}),
        )
        .await;
    let listed = stack.listed(&sent).await;
    assert_eq!(listed["channel"], "EMAIL");
    assert_eq!(listed["recipient"], "voter@example.org");
    assert_eq!(listed["content"]["subject"], "Step test message");
    assert_eq!(listed["report"]["state"], "NO_RECEIPTS");

    // Messenger: Get Started on the enrollment page's link binds it, the
    // backend sends the code, and its delivery is reported.
    let page_id = unique("");
    let username = unique("fake-page-");
    let messenger = stack
        .account(
            json!({
                "provider": "MESSENGER_SEND_API",
                "page_id": page_id,
                "page_name": "Elections",
                "page_username": username,
                "api_version": "v23.0",
            }),
            json!({"ACCESS_TOKEN": "synthetic-token", "APP_SECRET": APP_SECRET}),
            json!({"generate_verify_token": true}),
        )
        .await;
    // Every channel's check against the fake comes back Ready, for codes
    // and notices, as the event configuration judges it.
    assert_ready(
        &stack,
        &[
            (whatsapp.as_str(), MessageChannel::WHATSAPP),
            (viber.as_str(), MessageChannel::VIBER),
            (messenger.as_str(), MessageChannel::MESSENGER),
            (sms.as_str(), MessageChannel::SMS),
            (email.as_str(), MessageChannel::EMAIL),
        ],
    )
    .await;

    rows::execute(
        &stack.services.hasura,
        "INSERT INTO sequent_backend.messaging_event_config
             (tenant_id, election_event_id, config)
         VALUES ($1, $2, $3)",
        &[
            &Uuid::parse_str(&stack.event.tenant_id).unwrap(),
            &Uuid::parse_str(&stack.event.election_event_id).unwrap(),
            &json!({"version": 1, "channels": [{
                "channel": "MESSENGER", "account_id": messenger,
                "purposes": ["OTP"],
            }]}),
        ],
    )
    .await;
    let signer = RealmSigner::shared();
    stack.services.trust(&stack.event.tenant_id, signer);
    let keycloak = signer.sign(
        &Claims::new(&stack.event.tenant_id, "service-account-keycloak")
            .roles([Permissions::SERVICE_ACCOUNT])
            .azp(crate::services::service_auth::DEFAULT_SERVICE_CLIENT),
        &stack.event.tenant_id,
    );
    let (status, created) = stack
        .post_as(
            "/messages/link",
            &keycloak,
            json!({
                "tenant_id": stack.event.tenant_id,
                "election_event_id": stack.event.election_event_id,
                "auth_session": "session-digest",
                "challenge": "challenge-digest",
                "code": "482619",
                "language": "en",
                "content": {"text": "Your code is 482619."},
                "expires_at": (Utc::now() + chrono::Duration::minutes(5)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(status, 200, "{created}");
    let link = created["link"].as_str().unwrap().to_string();
    let psid = {
        let (page, reference) = parse_messenger_link(&link).unwrap();
        page_scoped_id(&page, &reference)
    };
    let get_started = |link: String| {
        let stack = &stack;
        async move {
            stack
                .post(
                    &format!("{BASE}/meta/messenger/get-started"),
                    json!({ "link": link }),
                )
                .await
        }
    };
    assert_eq!(
        get_started(link.clone()).await,
        (200, json!({"status": 200, "reason": null}))
    );
    let code_message = {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(message) = stack
                .messages()
                .await
                .into_iter()
                .find(|m| m["channel"] == "MESSENGER")
            {
                break message;
            }
            assert!(Instant::now() < deadline, "the code was not sent");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    assert_eq!(code_message["sender"], page_id.as_str());
    assert_eq!(code_message["recipient"], psid.as_str());
    assert_eq!(code_message["content"]["text"], "Your code is 482619.");
    // The link is marked once the provider has accepted the code, which the
    // provider's own list shows a moment earlier.
    let (state, bound): (String, Option<String>) = {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let rows = rows::query(
                &stack.services.hasura,
                "SELECT state, page_scoped_id FROM sequent_backend.messenger_link
                 WHERE tenant_id = $1",
                &[&Uuid::parse_str(&stack.event.tenant_id).unwrap()],
            )
            .await;
            let link: (String, Option<String>) =
                (rows[0].get(0), rows[0].get(1));
            if link.0 != "PENDING" || Instant::now() >= deadline {
                break link;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    assert_eq!(
        (state.as_str(), bound.as_deref()),
        ("CODE_SENT", Some(psid.as_str()))
    );
    let code_id: Uuid = rows::query(
        &stack.services.hasura,
        "SELECT id FROM sequent_backend.message WHERE provider_message_id = $1",
        &[&code_message["provider_message_id"].as_str().unwrap()],
    )
    .await[0]
        .get(0);
    stack.assert_delivered(&code_id).await;

    // The same link again: taken by the backend, and no other code is sent.
    assert_eq!(
        get_started(link.clone()).await,
        (200, json!({"status": 200, "reason": null}))
    );
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let codes = stack
        .messages()
        .await
        .into_iter()
        .filter(|m| m["channel"] == "MESSENGER")
        .count();
    assert_eq!(codes, 1);
    let (_, first_confirmed) = stack
        .post_as(
            "/messages/link/confirm",
            &keycloak,
            json!({
                "tenant_id": stack.event.tenant_id,
                "reference": created["reference"],
                "auth_session": "session-digest",
                "challenge": "challenge-digest",
            }),
        )
        .await;
    assert_eq!(first_confirmed["state"], "CONFIRMED");
    // A later sign-in link must use the voter's saved Page-scoped identity,
    // even though its one-time reference is different from enrollment's.
    let (status, sign_in) = stack
        .post_as(
            "/messages/link",
            &keycloak,
            json!({
                "tenant_id": stack.event.tenant_id,
                "election_event_id": stack.event.election_event_id,
                "auth_session": "later-sign-in-session",
                "challenge": "later-sign-in-challenge",
                "expected_recipient": psid,
                "expected_page": page_id,
                "code": "593720",
                "language": "en",
                "content": {"text": "Your code is 593720."},
                "expires_at": (Utc::now() + chrono::Duration::minutes(5)).to_rfc3339(),
            }),
        )
        .await;
    assert_eq!(status, 200, "{sign_in}");
    let later_link = sign_in["link"].as_str().unwrap().to_string();
    assert_ne!(later_link, link);
    assert_eq!(
        get_started(later_link).await,
        (200, json!({"status": 200, "reason": null}))
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(message) =
            stack.messages().await.into_iter().find(|message| {
                message["channel"] == "MESSENGER"
                    && message["content"]["text"] == "Your code is 593720."
            })
        {
            assert_eq!(message["recipient"], psid.as_str());
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the saved conversation's code was not sent"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let rows = loop {
        let rows = rows::query(
            &stack.services.hasura,
            "SELECT state, page_scoped_id FROM sequent_backend.messenger_link
             WHERE tenant_id = $1 AND auth_session_digest = $2",
            &[
                &Uuid::parse_str(&stack.event.tenant_id).unwrap(),
                &"later-sign-in-session",
            ],
        )
        .await;
        if rows[0].get::<_, String>(0) != "PENDING" {
            break rows;
        }
        assert!(
            Instant::now() < deadline,
            "the sign-in code was not committed"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(rows[0].get::<_, String>(0), "CODE_SENT");
    assert_eq!(
        rows[0].get::<_, Option<String>>(1).as_deref(),
        Some(psid.as_str())
    );
    let (status, confirmed) = stack
        .post_as(
            "/messages/link/confirm",
            &keycloak,
            json!({
                "tenant_id": stack.event.tenant_id,
                "reference": sign_in["reference"],
                "auth_session": "later-sign-in-session",
                "challenge": "later-sign-in-challenge",
            }),
        )
        .await;
    assert_eq!(status, 200, "{confirmed}");
    assert_eq!(confirmed["state"], "CONFIRMED");
    assert_eq!(confirmed["page_scoped_id"], psid.as_str());
    // Refusals, with their reasons.
    for (link, status, reason) in [
        (
            "https://example.org/x?ref=y".to_string(),
            400,
            LINK_UNREADABLE_REASON,
        ),
        (
            format!("https://m.me/{username}"),
            400,
            LINK_WITHOUT_REF_REASON,
        ),
        (
            "https://m.me/another-page?ref=y".to_string(),
            404,
            NO_PAGE_ACCOUNT_REASON,
        ),
    ] {
        assert_eq!(
            get_started(link).await,
            (status, json!({ "message": reason }))
        );
    }

    // Another tenant's administrator sees none of these messages.
    let other = Claims::new(&Uuid::new_v4().to_string(), USER_ID)
        .roles([Permissions::MESSAGING_ACCOUNT_WRITE]);
    let (status, list) = stack
        .post_as(&format!("{BASE}/messages"), &other.bearer(), json!({}))
        .await;
    assert_eq!(status, 200, "{list}");
    assert_eq!(list["messages"], json!([]));
    // And the list needs the account permission: `authorize` answers 401
    // without it, like the other messaging routes.
    let reader = Claims::new(&stack.event.tenant_id, USER_ID)
        .roles([Permissions::MESSAGING_CONFIG_READ]);
    let (status, _) = stack
        .post_as(&format!("{BASE}/messages"), &reader.bearer(), json!({}))
        .await;
    assert_eq!(status, 401);
}
