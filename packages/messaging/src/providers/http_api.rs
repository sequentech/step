// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A provider described by configuration: the account says which request
//! sends a message and how to read the answer, so a partner with a JSON
//! HTTP API needs no adapter of its own.

use super::{exchange, Account};
use crate::parameters::{parse_all, TemplateParameter};
use crate::sender::{
    outcome_from_http, ChannelSender, FailureKind, OutboundMessage, ProviderFailure, SendOutcome,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use hmac::{Hmac, Mac};
use openssl::base64::encode_block;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use sequent_core::types::messaging::{
    AccountCheck, AccountSender, CredentialName, HttpApiSender, HttpJwt, HttpRequestTemplate,
    HttpStatusMapping, JwtAlgorithm, MessageAttemptState, PhoneFormat, ProviderCapabilities,
    RecipientKind,
};
use serde_json::Value;
use sha2::Sha256;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const PARAMETERS_PLACEHOLDER: &str = "{{parameters}}";
const NAMED_PARAMETERS_PLACEHOLDER: &str = "{{named_parameters}}";

/// Values a request template may refer to.
#[derive(Debug, Default, Clone)]
pub struct Placeholders {
    values: BTreeMap<String, String>,
    parameters: Vec<String>,
    named_parameters: BTreeMap<String, String>,
}

fn base64_url(bytes: &[u8]) -> String {
    encode_block(bytes)
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string()
}

/// A compact JWT with `claims` plus `iat`, `exp` and `jti`.
fn mint_jwt(jwt: &HttpJwt, key: &str, claims: Value, message_id: &str) -> Result<String> {
    let now = Utc::now().timestamp();
    let mut claims = match claims {
        Value::Object(map) => map,
        Value::Null => serde_json::Map::new(),
        _ => return Err(anyhow!("JWT claims must be an object")),
    };
    claims.insert("iat".to_string(), Value::from(now));
    claims.insert(
        "exp".to_string(),
        Value::from(now + i64::try_from(jwt.lifetime_seconds).unwrap_or(300)),
    );
    claims.insert(
        "jti".to_string(),
        Value::from(format!("{message_id}-{}", crate::link::new_reference())),
    );
    let header = serde_json::json!({"alg": jwt.algorithm.to_string(), "typ": "JWT"});
    let signing_input = format!(
        "{}.{}",
        base64_url(header.to_string().as_bytes()),
        base64_url(Value::Object(claims).to_string().as_bytes())
    );
    let signature = match jwt.algorithm {
        JwtAlgorithm::HS256 => {
            let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes())
                .map_err(|error| anyhow!("invalid JWT secret: {error}"))?;
            mac.update(signing_input.as_bytes());
            mac.finalize().into_bytes().to_vec()
        }
        JwtAlgorithm::RS256 => {
            let private_key = PKey::private_key_from_pem(key.as_bytes())
                .context("API_SECRET is not a PEM private key")?;
            let mut signer = Signer::new(MessageDigest::sha256(), &private_key)?;
            signer.update(signing_input.as_bytes())?;
            signer.sign_to_vec()?
        }
    };
    Ok(format!("{signing_input}.{}", base64_url(&signature)))
}

impl Placeholders {
    pub fn set(&mut self, name: &str, value: impl Into<String>) {
        self.values.insert(name.to_string(), value.into());
    }

    /// Replaces every `{{name}}`; unknown names become empty.
    pub fn render(&self, template: &str) -> String {
        let mut out = String::with_capacity(template.len());
        let mut rest = template;
        while let Some(start) = rest.find("{{") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            match after.find("}}") {
                Some(end) => {
                    let name = after[..end].trim();
                    if let Some(value) = self.values.get(name) {
                        out.push_str(value);
                    }
                    rest = &after[end + 2..];
                }
                None => {
                    out.push_str(&rest[start..]);
                    rest = "";
                }
            }
        }
        out.push_str(rest);
        out
    }

    /// Renders every string in a JSON body. A string that is exactly
    /// `{{parameters}}` becomes the array of template parameters.
    pub fn render_json(&self, template: &Value) -> Value {
        match template {
            Value::String(s) if s.trim() == PARAMETERS_PLACEHOLDER => {
                Value::Array(self.parameters.iter().cloned().map(Value::String).collect())
            }
            Value::String(s) if s.trim() == NAMED_PARAMETERS_PLACEHOLDER => Value::Object(
                self.named_parameters
                    .iter()
                    .map(|(name, value)| (name.clone(), Value::String(value.clone())))
                    .collect(),
            ),
            Value::String(s) => Value::String(self.render(s)),
            Value::Array(items) => {
                Value::Array(items.iter().map(|v| self.render_json(v)).collect())
            }
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(key, value)| (key.clone(), self.render_json(value)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }
}

/// The value at a JSON pointer, as text.
pub fn pointer_text(value: &Value, pointer: &str) -> Option<String> {
    match value.pointer(pointer)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// The attempt state a provider's status value means, if it is mapped.
pub fn mapped_state(mapping: &HttpStatusMapping, item: &Value) -> Option<MessageAttemptState> {
    let status = pointer_text(item, &mapping.state_pointer)?;
    mapping
        .states
        .iter()
        .find(|(provider_value, _)| provider_value.eq_ignore_ascii_case(&status))
        .map(|(_, state)| *state)
}

type TokenCache = Mutex<HashMap<String, (String, Instant)>>;

fn token_cache() -> &'static TokenCache {
    static CACHE: OnceLock<TokenCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct HttpApiChannelSender {
    account: Account,
    api: HttpApiSender,
    http: reqwest::Client,
}

impl HttpApiChannelSender {
    pub fn new(account: Account, http: reqwest::Client) -> Result<Self> {
        let AccountSender::HTTP_API(api) = &account.sender else {
            return Err(anyhow!("not a configurable HTTP account"));
        };
        if !api.send.url.starts_with("https://") && !api.send.url.starts_with("http://") {
            return Err(anyhow!("the send request needs an http(s) URL"));
        }
        Ok(HttpApiChannelSender {
            api: api.clone(),
            account,
            http,
        })
    }

    fn base_placeholders(&self) -> Placeholders {
        let mut placeholders = Placeholders::default();
        for (name, value) in &self.account.credentials {
            placeholders.set(&format!("credential.{name}"), value.clone());
        }
        if let (Some(username), Some(password)) = (
            self.account.credentials.get(&CredentialName::USERNAME),
            self.account.credentials.get(&CredentialName::PASSWORD),
        ) {
            placeholders.set(
                "basic_auth",
                encode_block(format!("{username}:{password}").as_bytes()),
            );
        }
        if let Some(callback) = &self.account.callback_url {
            placeholders.set("callback_url", callback.clone());
        }
        placeholders
    }

    fn request(
        &self,
        template: &HttpRequestTemplate,
        placeholders: &Placeholders,
    ) -> Result<reqwest::RequestBuilder> {
        let method = reqwest::Method::from_bytes(template.method.to_uppercase().as_bytes())
            .context("invalid HTTP method")?;
        let mut request = self
            .http
            .request(method, placeholders.render(&template.url));
        for (name, value) in &template.headers {
            request = request.header(name, placeholders.render(value));
        }
        if let Some(body) = &template.body {
            request = request.json(&placeholders.render_json(body));
        }
        Ok(request)
    }

    /// Adds `{{token}}` when the account exchanges credentials for one.
    async fn with_token(&self, mut placeholders: Placeholders) -> Result<Placeholders> {
        let Some(token_request) = &self.api.token else {
            return Ok(placeholders);
        };
        let cached = token_cache().lock().ok().and_then(|cache| {
            cache
                .get(&self.account.id)
                .filter(|(_, expires)| *expires > Instant::now())
                .map(|(token, _)| token.clone())
        });
        let token = match cached {
            Some(token) => token,
            None => {
                let (status, body) =
                    exchange(self.request(&token_request.request, &placeholders)?).await?;
                if !(200..300).contains(&status) {
                    return Err(anyhow!("token request failed with status {status}"));
                }
                let token = pointer_text(
                    &serde_json::from_str::<Value>(&body)?,
                    &token_request.token_pointer,
                )
                .ok_or_else(|| anyhow!("token response has no token"))?;
                if let Ok(mut cache) = token_cache().lock() {
                    cache.insert(
                        self.account.id.clone(),
                        (
                            token.clone(),
                            Instant::now() + Duration::from_secs(token_request.lifetime_seconds),
                        ),
                    );
                }
                token
            }
        };
        placeholders.set("token", token);
        Ok(placeholders)
    }

    fn message_placeholders(&self, message: &OutboundMessage) -> Placeholders {
        let mut placeholders = self.base_placeholders();
        let to = match (message.destination.kind, self.api.phone_format) {
            (RecipientKind::PHONE_NUMBER, PhoneFormat::DIGITS) => message
                .destination
                .as_str()
                .trim_start_matches('+')
                .to_string(),
            _ => message.destination.as_str().to_string(),
        };
        placeholders.set("to", to);
        placeholders.set("text", message.content.text.clone());
        placeholders.set(
            "subject",
            message.content.subject.clone().unwrap_or_default(),
        );
        placeholders.set("html", message.content.html.clone().unwrap_or_default());
        placeholders.set("code", message.content.code.clone().unwrap_or_default());
        placeholders.set(
            "template",
            message.provider_template.clone().unwrap_or_default(),
        );
        placeholders.set("language", message.language.clone().unwrap_or_default());
        placeholders.set("message_id", message.idempotency_key.clone());
        for parameter in parse_all(&message.content.template_parameters) {
            match parameter {
                TemplateParameter::Positional(value) => {
                    placeholders.parameters.push(value.clone());
                    let index = placeholders.parameters.len();
                    placeholders.set(&format!("param.{index}"), value);
                }
                TemplateParameter::Named { name, value } => {
                    placeholders.set(&format!("param.{name}"), value.clone());
                    placeholders.named_parameters.insert(name, value);
                }
            }
        }
        placeholders
    }

    /// Adds `{{jwt}}` when the account signs its requests with one.
    fn with_jwt(&self, mut placeholders: Placeholders, message_id: &str) -> Result<Placeholders> {
        if let Some(jwt) = &self.api.jwt {
            let key = self.account.credential(CredentialName::API_SECRET)?;
            let claims = placeholders.render_json(&jwt.claims);
            placeholders.set("jwt", mint_jwt(jwt, key, claims, message_id)?);
        }
        Ok(placeholders)
    }

    /// Placeholders with the account's token and JWT, when it uses them.
    async fn authenticated(
        &self,
        placeholders: Placeholders,
        message_id: &str,
    ) -> Result<Placeholders> {
        self.with_jwt(self.with_token(placeholders).await?, message_id)
    }
}

fn refused(reason: impl ToString) -> SendOutcome {
    SendOutcome::Rejected(ProviderFailure {
        kind: FailureKind::PERMANENT,
        code: None,
        reason: reason.to_string(),
    })
}

#[async_trait]
impl ChannelSender for HttpApiChannelSender {
    fn capabilities(&self) -> ProviderCapabilities {
        self.account
            .sender
            .capabilities(self.account.channel)
            .unwrap_or_else(|| unreachable!("a configurable provider accepts every channel"))
    }

    async fn send(&self, message: &OutboundMessage) -> SendOutcome {
        // Nothing has been sent yet if the token or the request cannot be
        // prepared.
        let placeholders = match self
            .authenticated(self.message_placeholders(message), &message.idempotency_key)
            .await
        {
            Ok(placeholders) => placeholders,
            Err(_) => {
                return SendOutcome::Rejected(ProviderFailure {
                    kind: FailureKind::TRANSIENT,
                    code: None,
                    reason: "could not obtain the provider token".to_string(),
                })
            }
        };
        let request = match self.request(&self.api.send, &placeholders) {
            Ok(request) => request,
            Err(error) => return refused(error),
        };
        let pointer = self.api.message_id_pointer.clone();
        let key = message.idempotency_key.clone();
        outcome_from_http(
            exchange(request).await,
            |body| {
                pointer
                    .as_deref()
                    .and_then(|pointer| {
                        pointer_text(&serde_json::from_str::<Value>(body).ok()?, pointer)
                    })
                    .or_else(|| Some(key.clone()))
            },
            |_| None,
        )
    }

    async fn check(&self) -> AccountCheck {
        let now = Some(Utc::now().to_rfc3339());
        let approved_templates = self.api.approved_templates.clone();
        let Some(check) = &self.api.check else {
            return AccountCheck {
                connected: false,
                production_access: false,
                approved_templates,
                checked_at: now,
                reason: Some(
                    "no check request is configured; confirm readiness as an administrator"
                        .to_string(),
                ),
            };
        };
        let result = match self.authenticated(self.base_placeholders(), "check").await {
            Ok(placeholders) => match self.request(check, &placeholders) {
                Ok(request) => exchange(request)
                    .await
                    .map_err(|_| "provider unreachable".to_string()),
                Err(error) => Err(error.to_string()),
            },
            Err(_) => Err("could not obtain the provider token".to_string()),
        };
        let reason = match result {
            Ok((status, _)) if (200..300).contains(&status) => None,
            Ok((status, _)) => Some(format!("the check request answered with status {status}")),
            Err(reason) => Some(reason),
        };
        AccountCheck {
            connected: reason.is_none(),
            production_access: reason.is_none(),
            approved_templates,
            checked_at: now,
            reason,
        }
    }

    async fn reconcile(&self, provider_message_id: &str) -> Option<MessageAttemptState> {
        let reconcile = self.api.reconcile.as_ref()?;
        let mut placeholders = self
            .authenticated(self.base_placeholders(), provider_message_id)
            .await
            .ok()?;
        placeholders.set("message_id", provider_message_id);
        let (status, body) = exchange(self.request(&reconcile.request, &placeholders).ok()?)
            .await
            .ok()?;
        if !(200..300).contains(&status) {
            return None;
        }
        mapped_state(
            &reconcile.status,
            &serde_json::from_str::<Value>(&body).ok()?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::destination::Destination;
    use crate::test_server::{Reply, TestServer};
    use sequent_core::types::messaging::{
        AccountLimits, MessageChannel, MessageContent, MessagePurpose, OutOfWindowPolicy,
    };
    use serde_json::json;

    fn account(base_url: &str, extra: Value) -> Account {
        let mut sender = json!({
            "provider": "HTTP_API",
            "label": "COMELEC",
            "send": {
                "url": format!("{base_url}/v1/messages/{{{{to}}}}"),
                "headers": {
                    "Authorization": "Bearer {{credential.API_KEY}}",
                    "X-Basic": "Basic {{basic_auth}}"
                },
                "body": {
                    "to": "{{to}}",
                    "channel": "viber",
                    "template": {"id": "{{template}}", "lang": "{{language}}", "params": "{{parameters}}"},
                    "text": "{{text}}",
                    "first": "{{param.1}}",
                    "pin": "{{code}}",
                    "ref": "{{message_id}}",
                    "callback": "{{callback_url}}",
                    "ttl": 300
                }
            },
            "message_id_pointer": "/data/0/id",
            "phone_format": "DIGITS",
            "template_required_for": ["OTP"]
        });
        for (key, value) in extra.as_object().cloned().unwrap_or_default() {
            sender[key] = value;
        }
        Account {
            id: format!("http-{base_url}"),
            channel: MessageChannel::VIBER,
            sender: serde_json::from_value(sender).expect("sender"),
            credentials: BTreeMap::from([
                (CredentialName::API_KEY, "key-1".to_string()),
                (CredentialName::USERNAME, "user".to_string()),
                (CredentialName::PASSWORD, "pass".to_string()),
            ]),
            limits: AccountLimits::default(),
            callback_url: Some("https://step.example/webhooks/http/abc".to_string()),
        }
    }

    fn message() -> OutboundMessage {
        OutboundMessage {
            channel: MessageChannel::VIBER,
            purpose: MessagePurpose::OTP,
            destination: Destination::parse(MessageChannel::VIBER, "+639171234567").unwrap(),
            language: Some("en".to_string()),
            content: MessageContent {
                subject: None,
                text: "Your code is 482619".to_string(),
                html: None,
                template_parameters: vec!["482619".to_string(), "5".to_string()],
                code: Some("482619".to_string()),
            },
            provider_template: Some("otp_tpl".to_string()),
            idempotency_key: "logical-1:1".to_string(),
            expires_at: Some(Utc::now() + chrono::Duration::minutes(5)),
            last_inbound_at: None,
            out_of_window: OutOfWindowPolicy::DISABLED,
        }
    }

    #[test]
    fn placeholders_fill_strings_and_the_parameter_array() {
        let mut placeholders = Placeholders::default();
        placeholders.set("to", "639171234567");
        placeholders.parameters = vec!["a".to_string(), "b".to_string()];
        assert_eq!(
            placeholders.render("x {{to}} {{ to }} {{unknown}} y"),
            "x 639171234567 639171234567  y"
        );
        assert_eq!(placeholders.render("broken {{to"), "broken {{to");
        assert_eq!(
            placeholders.render_json(&json!({"a": ["{{to}}", 1, true], "p": "{{parameters}}"})),
            json!({"a": ["639171234567", 1, true], "p": ["a", "b"]})
        );
    }

    #[tokio::test]
    async fn the_configured_request_is_sent_and_the_message_id_read() {
        let server =
            TestServer::start(vec![Reply::Json(202, json!({"data": [{"id": 9001}]}))]).await;
        let sender =
            HttpApiChannelSender::new(account(&server.base_url, json!({})), reqwest::Client::new())
                .unwrap();
        assert_eq!(
            sender.send(&message()).await,
            SendOutcome::Accepted {
                provider_message_id: Some("9001".to_string())
            }
        );
        let request = &server.requests()[0];
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/v1/messages/639171234567");
        assert_eq!(request.header("authorization"), Some("Bearer key-1"));
        assert_eq!(request.header("x-basic"), Some("Basic dXNlcjpwYXNz"));
        assert_eq!(
            request.json(),
            json!({
                "to": "639171234567",
                "channel": "viber",
                "template": {"id": "otp_tpl", "lang": "en", "params": ["482619", "5"]},
                "text": "Your code is 482619",
                "first": "482619",
                "pin": "482619",
                "ref": "logical-1:1",
                "callback": "https://step.example/webhooks/http/abc",
                "ttl": 300
            })
        );
        assert_eq!(
            sender.capabilities().template_required_for,
            vec![MessagePurpose::OTP]
        );
    }

    #[tokio::test]
    async fn refusals_and_lost_answers_keep_their_meaning() {
        let server =
            TestServer::start(vec![Reply::Json(422, json!({"error": "bad"})), Reply::Drop]).await;
        let sender =
            HttpApiChannelSender::new(account(&server.base_url, json!({})), reqwest::Client::new())
                .unwrap();
        assert_eq!(
            sender.send(&message()).await.state(),
            MessageAttemptState::FAILED
        );
        assert_eq!(
            sender.send(&message()).await.state(),
            MessageAttemptState::UNKNOWN
        );
    }

    #[tokio::test]
    async fn a_token_is_exchanged_once_and_reused() {
        let server = TestServer::start(vec![
            Reply::Json(200, json!({"access_token": "tok-1"})),
            Reply::Json(200, json!({"data": [{"id": "m1"}]})),
            Reply::Json(200, json!({"data": [{"id": "m2"}]})),
        ])
        .await;
        let extra = json!({
            "token": {
                "request": {
                    "url": format!("{}/oauth/token", server.base_url),
                    "body": {"client_id": "{{credential.USERNAME}}", "client_secret": "{{credential.PASSWORD}}"}
                },
                "token_pointer": "/access_token"
            }
        });
        let mut account = account(&server.base_url, extra);
        if let AccountSender::HTTP_API(api) = &mut account.sender {
            api.send
                .headers
                .insert("Authorization".to_string(), "Bearer {{token}}".to_string());
        }
        let sender = HttpApiChannelSender::new(account, reqwest::Client::new()).unwrap();
        sender.send(&message()).await;
        sender.send(&message()).await;
        let requests = server.requests();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].path, "/oauth/token");
        assert_eq!(requests[0].json()["client_secret"], "pass");
        assert_eq!(requests[1].header("authorization"), Some("Bearer tok-1"));
        assert_eq!(requests[2].header("authorization"), Some("Bearer tok-1"));
    }

    #[tokio::test]
    async fn checks_and_reconciliation_use_their_configured_requests() {
        let server = TestServer::start(vec![
            Reply::Json(200, json!({"ok": true})),
            Reply::Json(401, json!({})),
            Reply::Json(200, json!({"message": {"status": "Delivered"}})),
        ])
        .await;
        let extra = json!({
            "check": {"method": "GET", "url": format!("{}/v1/account", server.base_url),
                      "headers": {"Authorization": "Bearer {{credential.API_KEY}}"}},
            "reconcile": {
                "request": {"method": "GET", "url": format!("{}/v1/messages/{{{{message_id}}}}", server.base_url)},
                "status": {"message_id_pointer": "/message/id", "state_pointer": "/message/status",
                           "states": {"delivered": "DELIVERED", "failed": "FAILED"}}
            },
            "approved_templates": {"OTP": ["en"]}
        });
        let sender =
            HttpApiChannelSender::new(account(&server.base_url, extra), reqwest::Client::new())
                .unwrap();
        let check = sender.check().await;
        assert!(check.connected);
        assert_eq!(
            check.approved_templates[&MessagePurpose::OTP],
            vec!["en".to_string()]
        );
        assert!(!sender.check().await.connected);
        assert_eq!(
            sender.reconcile("m-7").await,
            Some(MessageAttemptState::DELIVERED)
        );
        assert_eq!(server.requests()[2].path, "/v1/messages/m-7");
        assert!(sender.capabilities().reconciliation);
    }

    #[tokio::test]
    async fn requests_can_carry_a_minted_jwt() {
        let server =
            TestServer::start(vec![Reply::Json(200, json!({"data": [{"id": "m1"}]}))]).await;
        let extra = json!({"jwt": {"algorithm": "HS256", "claims": {"application_id": "app-{{credential.API_KEY}}"}}});
        let mut account = account(&server.base_url, extra);
        account
            .credentials
            .insert(CredentialName::API_SECRET, "signing-secret".to_string());
        if let AccountSender::HTTP_API(api) = &mut account.sender {
            api.send
                .headers
                .insert("Authorization".to_string(), "Bearer {{jwt}}".to_string());
        }
        let sender = HttpApiChannelSender::new(account, reqwest::Client::new()).unwrap();
        sender.send(&message()).await;
        let request = &server.requests()[0];
        let header = request.header("authorization").unwrap().to_string();
        let auth = sequent_core::types::messaging::HttpWebhookAuth::JWT_HS256 {
            header: "authorization".to_string(),
        };
        let headers = BTreeMap::from([("authorization".to_string(), header.clone())]);
        assert!(crate::webhooks::http::verify(
            &auth,
            Some("signing-secret"),
            &headers,
            b""
        ));
        assert!(!crate::webhooks::http::verify(
            &auth,
            Some("other"),
            &headers,
            b""
        ));
        let claims = header.split('.').nth(1).unwrap();
        let mut padded = claims.replace('-', "+").replace('_', "/");
        while padded.len() % 4 != 0 {
            padded.push('=');
        }
        let claims: Value =
            serde_json::from_slice(&openssl::base64::decode_block(&padded).unwrap()).unwrap();
        assert_eq!(claims["application_id"], "app-key-1");
        assert!(claims["exp"].as_i64().unwrap() > claims["iat"].as_i64().unwrap());
    }

    #[tokio::test]
    async fn an_account_without_a_check_request_is_not_reported_connected() {
        let sender = HttpApiChannelSender::new(
            account("http://127.0.0.1:9", json!({})),
            reqwest::Client::new(),
        )
        .unwrap();
        let check = sender.check().await;
        assert!(!check.connected);
        assert!(check.reason.unwrap().contains("confirm readiness"));
    }
}
