// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Credential-free sender identities bound by a signed delivery. A receiver
//! maps these references to target-owned accounts; no authorization travels.
use crate::types::messaging::{
    AccountCheck, AccountCheckDetails, AccountSender, CheckSource,
    EventMessagingConfig, MessageChannel, MessagePurpose, ProviderApproval,
    ViberContentMode,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MEMBER: &str = "tenant_messaging_accounts.json";
pub const REF_PREFIX: &str = "package:";
pub const FORMAT: &str = "sequent.tenant-messaging-accounts/1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortableMessagingFormat {
    #[serde(rename = "sequent.tenant-messaging-accounts/1")]
    V1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableMessagingAccounts {
    pub format: PortableMessagingFormat,
    pub accounts: Vec<PortableMessagingAccount>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableMessagingAccount {
    pub reference: String,
    pub name: String,
    pub sender: PortableSender,
    /// A hiding commitment to the account's credential, which travels
    /// beside the signed delivery, never inside it (see
    /// `messaging_credentials`): a static key pair for AWS SES/SNS, an API
    /// key for Infobip (Viber, WhatsApp). Absent: the receiver keeps the
    /// account's existing credentials (for a new AWS account, the
    /// service's workload role; a new Infobip account waits for its key).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_commitment: Option<String>,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "provider", deny_unknown_fields)]
pub enum PortableSender {
    WHATSAPP_CLOUD_API {
        business_account_id: String,
        phone_number_id: String,
        display_phone_number: String,
        display_name: Option<String>,
        api_version: String,
    },
    MESSENGER_SEND_API {
        page_id: String,
        page_name: Option<String>,
        page_username: Option<String>,
        api_version: String,
    },
    VIBER_INFOBIP {
        base_url: String,
        sender: String,
        /// Absent: the receiver's default (approved templates), or the
        /// account's current mode when it already exists.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        content_mode: Option<ViberContentMode>,
        /// Template IDs the partner approved, per purpose, as language to
        /// template ID: public, and not readable from Infobip. Absent: an
        /// existing account keeps the approvals entered on it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        approved_templates:
            Option<BTreeMap<MessagePurpose, BTreeMap<String, String>>>,
    },
    WHATSAPP_INFOBIP {
        base_url: String,
        /// The WhatsApp number registered with Infobip, in international
        /// format, with or without `+`.
        sender_number: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
    },
    AWS_SNS {
        sender_id: Option<String>,
        origination_number: Option<String>,
        region: Option<String>,
    },
    AWS_SES {
        from_address: String,
        from_name: Option<String>,
        region: Option<String>,
    },
}

/// The kind of credential a sender's declaration may commit to.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveredCredentialKind {
    /// A long-term AWS access key ID and secret (AWS SES and SNS).
    AWS_KEY_PAIR,
    /// An Infobip API key (Viber and WhatsApp through Infobip).
    API_KEY,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidPortableMessaging;
impl std::fmt::Display for InvalidPortableMessaging {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid portable messaging account declarations")
    }
}
impl std::error::Error for InvalidPortableMessaging {}

impl PortableMessagingAccounts {
    pub fn parse(bytes: &[u8]) -> Result<Self, InvalidPortableMessaging> {
        parse(bytes)
    }
    pub fn validate(&self) -> Result<(), InvalidPortableMessaging> {
        let mut used = BTreeSet::new();
        let mut names = BTreeSet::new();
        let mut identities = Vec::new();
        if !(1..=32).contains(&self.accounts.len())
            || !self.accounts.iter().all(|a| {
                slug(&a.reference, 48)
                    && used.insert(&a.reference)
                    && label(&a.name)
                    && a.sender.valid()
                    && a.credential_commitment_valid()
                    && names.insert(a.name.trim().to_lowercase())
                    && {
                        let identity = a.account_sender().identity();
                        let distinct = !identities.contains(&identity);
                        identities.push(identity);
                        distinct
                    }
            })
        {
            return Err(InvalidPortableMessaging);
        }
        Ok(())
    }
}

pub fn parse(
    bytes: &[u8],
) -> Result<PortableMessagingAccounts, InvalidPortableMessaging> {
    if bytes.len() > 64 * 1024 {
        return Err(InvalidPortableMessaging);
    }
    let parsed: PortableMessagingAccounts =
        serde_json::from_slice(bytes).map_err(|_| InvalidPortableMessaging)?;
    parsed.validate()?;
    Ok(parsed)
}

/// Per account reference (`package:<ref>`) and purpose, the template
/// languages a configuration package names for it.
pub type DeclaredLanguages =
    BTreeMap<String, BTreeMap<MessagePurpose, BTreeSet<String>>>;

impl PortableMessagingAccounts {
    /// The template languages the package names for each account: its event's
    /// template settings on the account's channel (each binding's language and
    /// the provider's code for it), Viber's declared template IDs, and, for
    /// notices, the provider languages of the package's message templates on
    /// the account's channel (`notice_templates`).
    pub fn declared_languages(
        &self,
        event: Option<&EventMessagingConfig>,
        notice_templates: &[(MessageChannel, String)],
    ) -> DeclaredLanguages {
        let mut declared = DeclaredLanguages::new();
        for account in &self.accounts {
            let reference = account.reference_key();
            let mut languages: BTreeMap<MessagePurpose, BTreeSet<String>> =
                BTreeMap::new();
            if let PortableSender::VIBER_INFOBIP {
                approved_templates: Some(templates),
                ..
            } = &account.sender
            {
                for (purpose, by_language) in templates {
                    languages
                        .entry(*purpose)
                        .or_default()
                        .extend(by_language.keys().cloned());
                }
            }
            let channels = event
                .map(|event| event.channels.as_slice())
                .unwrap_or_default();
            for channel in channels.iter().filter(|c| c.account_id == reference)
            {
                for binding in &channel.templates {
                    let entry = languages.entry(binding.purpose).or_default();
                    entry.insert(binding.language.clone());
                    entry.insert(binding.provider_language().to_string());
                }
            }
            let channel = account.account_sender().provider().channel();
            for (template_channel, language) in notice_templates {
                if Some(*template_channel) == channel {
                    languages
                        .entry(MessagePurpose::NOTICE)
                        .or_default()
                        .insert(language.clone());
                }
            }
            declared.insert(reference, languages);
        }
        declared
    }
}

impl PortableMessagingAccount {
    pub fn reference_key(&self) -> String {
        format!("{REF_PREFIX}{}", self.reference)
    }
    /// Only senders that take a delivered credential may commit to one, and
    /// only as a SHA-256 commitment: 64 lowercase hexadecimal digits, never
    /// key material.
    fn credential_commitment_valid(&self) -> bool {
        match &self.credential_commitment {
            None => true,
            Some(c) => {
                self.sender.delivered_credential().is_some()
                    && c.len() == 64
                    && c.bytes().all(|b| {
                        b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
                    })
            }
        }
    }
    /// The account as its package's author checked it: connected, with
    /// production access and the provider's approval, and approved templates
    /// in the languages the package names for it ([`PortableMessagingAccounts::declared_languages`]).
    /// A receiver takes it as checked at `checked_at`, for the account's own
    /// identity, until an administrator checks it with the provider.
    pub fn declared_check(
        &self,
        languages: &DeclaredLanguages,
        checked_at: String,
    ) -> AccountCheck {
        let sender = self.account_sender();
        AccountCheck {
            connected: true,
            production_access: true,
            approved_templates: languages
                .get(&self.reference_key())
                .map(|by_purpose| {
                    by_purpose
                        .iter()
                        .filter(|(_, languages)| !languages.is_empty())
                        .map(|(purpose, languages)| {
                            (*purpose, languages.iter().cloned().collect())
                        })
                        .collect()
                })
                .unwrap_or_default(),
            checked_at: Some(checked_at),
            reason: None,
            details: AccountCheckDetails {
                identity: Some(sender.identity()),
                provider_approval: Some(ProviderApproval::CONFIRMED),
                source: CheckSource::CONFIGURATION_PACKAGE,
                ..Default::default()
            },
        }
    }
    pub fn account_sender(&self) -> AccountSender {
        use PortableSender::*;
        match &self.sender {
            WHATSAPP_CLOUD_API {
                business_account_id,
                phone_number_id,
                display_phone_number,
                display_name,
                api_version,
            } => AccountSender::WHATSAPP_CLOUD_API {
                business_account_id: business_account_id.clone(),
                phone_number_id: phone_number_id.clone(),
                display_phone_number: display_phone_number.clone(),
                display_name: display_name.clone(),
                api_version: api_version.clone(),
                api_base_url: None,
            },
            MESSENGER_SEND_API {
                page_id,
                page_name,
                page_username,
                api_version,
            } => AccountSender::MESSENGER_SEND_API {
                page_id: page_id.clone(),
                page_name: page_name.clone(),
                page_username: page_username.clone(),
                api_version: api_version.clone(),
                api_base_url: None,
            },
            VIBER_INFOBIP {
                base_url,
                sender,
                content_mode,
                approved_templates,
            } => AccountSender::VIBER_INFOBIP {
                base_url: base_url.clone(),
                sender: sender.clone(),
                content_mode: content_mode.unwrap_or_default(),
                approved_templates: approved_templates
                    .clone()
                    .unwrap_or_default(),
            },
            WHATSAPP_INFOBIP {
                base_url,
                sender_number,
                display_name,
            } => AccountSender::WHATSAPP_INFOBIP {
                base_url: base_url.clone(),
                sender_number: sender_number.clone(),
                display_name: display_name.clone(),
            },
            AWS_SNS {
                sender_id,
                origination_number,
                region,
            } => AccountSender::AWS_SNS {
                sender_id: sender_id.clone(),
                origination_number: origination_number.clone(),
                region: region.clone(),
            },
            AWS_SES {
                from_address,
                from_name,
                region,
            } => AccountSender::AWS_SES {
                from_address: from_address.clone(),
                from_name: from_name.clone(),
                region: region.clone(),
                notification_topic_arn: None,
            },
        }
    }
}

fn slug(s: &str, max: usize) -> bool {
    s.len() <= max
        && s.as_bytes().first().is_some_and(|b| b.is_ascii_lowercase())
        && s.as_bytes()
            .last()
            .is_some_and(|b| b.is_ascii_alphanumeric())
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn label(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.trim() == s
        && s.chars()
            .all(|c| c.is_alphanumeric() || " .,_-()&'+".contains(c))
}
fn digits(s: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}
fn phone(s: &str) -> bool {
    s.strip_prefix('+')
        .is_some_and(|v| digits(v, 5, 15) && !v.starts_with('0'))
}
fn graph_version(s: &str) -> bool {
    s.strip_prefix('v')
        .and_then(|v| v.split_once('.'))
        .is_some_and(|(major, minor)| {
            digits(major, 1, 3) && digits(minor, 1, 2)
        })
}
fn infobip(s: &str) -> bool {
    let Some(host) = s.strip_prefix("https://") else {
        return false;
    };
    let host = host.strip_suffix('/').unwrap_or(host);
    host == "api.infobip.com"
        || host
            .strip_suffix(".api.infobip.com")
            .is_some_and(|sub| slug(sub, 63))
}
fn region(s: &str) -> bool {
    slug(s, 32)
        && s.split('-').count() >= 3
        && s.rsplit('-').next().is_some_and(|last| digits(last, 1, 2))
}
fn email(s: &str) -> bool {
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    s.len() <= 128
        && !local.is_empty()
        && local.len() <= 64
        && local
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._+-".contains(&b))
        && domain.contains('.')
        && domain.split('.').all(|part| {
            !part.is_empty()
                && part.len() <= 63
                && part
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && part
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}
/// A language tag, as Viber templates are keyed: `en`, `pt-BR`, `es_419`.
fn language(s: &str) -> bool {
    (2..=16).contains(&s.len())
        && s.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn template_id(s: &str) -> bool {
    (1..=128).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}
impl PortableSender {
    /// The credential a declaration for this sender may commit to. `None`:
    /// its credentials are only ever entered on the receiving system.
    pub fn delivered_credential(&self) -> Option<DeliveredCredentialKind> {
        use PortableSender::*;
        match self {
            AWS_SES { .. } | AWS_SNS { .. } => {
                Some(DeliveredCredentialKind::AWS_KEY_PAIR)
            }
            VIBER_INFOBIP { .. } | WHATSAPP_INFOBIP { .. } => {
                Some(DeliveredCredentialKind::API_KEY)
            }
            WHATSAPP_CLOUD_API { .. } | MESSENGER_SEND_API { .. } => None,
        }
    }

    fn valid(&self) -> bool {
        use PortableSender::*;
        match self {
            WHATSAPP_CLOUD_API {
                business_account_id,
                phone_number_id,
                display_phone_number,
                display_name,
                api_version,
            } => {
                digits(business_account_id, 1, 32)
                    && digits(phone_number_id, 1, 32)
                    && phone(display_phone_number)
                    && display_name.as_deref().is_none_or(label)
                    && graph_version(api_version)
            }
            MESSENGER_SEND_API {
                page_id,
                page_name,
                page_username,
                api_version,
            } => {
                digits(page_id, 1, 32)
                    && page_name.as_deref().is_none_or(label)
                    && page_username.as_deref().is_none_or(|s| {
                        !s.is_empty()
                            && s.len() <= 64
                            && s.bytes().all(|b| {
                                b.is_ascii_alphanumeric() || b"._".contains(&b)
                            })
                    })
                    && graph_version(api_version)
            }
            VIBER_INFOBIP {
                base_url,
                sender,
                content_mode: _,
                approved_templates,
            } => {
                infobip(base_url)
                    && label(sender)
                    && approved_templates.as_ref().is_none_or(|purposes| {
                        purposes.values().all(|languages| {
                            languages
                                .iter()
                                .all(|(l, id)| language(l) && template_id(id))
                        })
                    })
            }
            WHATSAPP_INFOBIP {
                base_url,
                sender_number,
                display_name,
            } => {
                let number =
                    sender_number.strip_prefix('+').unwrap_or(sender_number);
                infobip(base_url)
                    && digits(number, 5, 15)
                    && !number.starts_with('0')
                    && display_name.as_deref().is_none_or(label)
            }
            AWS_SNS {
                sender_id,
                origination_number,
                region: r,
            } => {
                sender_id.as_deref().is_none_or(|s| {
                    (1..=11).contains(&s.len())
                        && s.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b' ')
                }) && origination_number
                    .as_deref()
                    .is_none_or(|s| phone(s) || digits(s, 3, 8))
                    && r.as_deref().is_none_or(region)
            }
            AWS_SES {
                from_address,
                from_name,
                region: r,
            } => {
                email(from_address)
                    && from_name.as_deref().is_none_or(label)
                    && r.as_deref().is_none_or(region)
            }
        }
    }
}

pub(super) fn problem() -> super::problem::Problem {
    super::problem::Problem::error(
        super::problem::Code::InvalidValue,
        "tenant_package.messagingAccounts",
        "expected credential-free portable messaging account declarations with distinct references, names and supported sender identities",
    )
}

pub(super) fn validate_value(
    value: &serde_json::Value,
) -> Result<(), super::problem::Problem> {
    if value.is_null() {
        return Ok(());
    }
    let typed: PortableMessagingAccounts =
        serde_json::from_value(value.clone()).map_err(|_| problem())?;
    typed.validate().map_err(|_| problem())
}

/// Check the credential-free declarations against the signed event's exact
/// set of package references. Legacy target-local IDs are not rewritten or
/// interpreted here. Each package reference must name a sender for its channel.
pub fn validate_event_references(
    definitions: Option<&PortableMessagingAccounts>,
    config: Option<&crate::types::messaging::EventMessagingConfig>,
) -> Result<(), super::problem::Problem> {
    use super::problem::{Code, Problem};
    if let Some(definitions) = definitions {
        definitions.validate().map_err(|_| problem())?;
    }
    let Some(config) = config else {
        return if definitions.is_some() {
            Err(Problem::error(
                Code::MissingField,
                "messaging_config",
                "portable messaging accounts require an event messaging configuration",
            ))
        } else {
            Ok(())
        };
    };
    let mut used = BTreeSet::new();
    for channel in &config.channels {
        if let Some(reference) = channel.account_id.strip_prefix(REF_PREFIX) {
            if !definitions.is_some_and(|bundle| {
                bundle.accounts.iter().any(|a| {
                    a.reference == reference
                        && a.account_sender().provider().channel()
                            == Some(channel.channel)
                })
            }) {
                return Err(Problem::error(
                    Code::DanglingReference,
                    "messaging_config.channels.account_id",
                    "a portable account reference has no matching sender for that channel",
                ));
            }
            used.insert(reference);
        }
    }
    if definitions.is_some_and(|bundle| {
        bundle
            .accounts
            .iter()
            .any(|account| !used.contains(account.reference.as_str()))
    }) {
        return Err(Problem::error(
            Code::DanglingReference,
            "tenant_package.messagingAccounts",
            "every declared portable messaging account must be used by the event",
        ));
    }
    Ok(())
}

fn event_config(
    config: Option<&serde_json::Value>,
) -> Result<
    Option<crate::types::messaging::EventMessagingConfig>,
    super::problem::Problem,
> {
    config
        .filter(|v| !v.is_null())
        .map(|v| {
            serde_json::from_value(v.clone()).map_err(|_| {
                super::problem::Problem::error(
                    super::problem::Code::InvalidValue,
                    "messaging_config",
                    "expected a readable event messaging configuration",
                )
            })
        })
        .transpose()
}

/// Check at every ordinary source-to-save boundary, including raw Blueprints.
/// A separately authorized profile may add declarations later, so when there
/// are none in the workbook, exact binding waits until the combined delivery.
pub(super) fn validate_workbook_sheets(
    sheets: &[super::sheet::Sheet],
    base: Option<&serde_json::Value>,
) -> Result<(), super::problem::Problem> {
    let rows: Vec<_> = sheets
        .iter()
        .filter(|sheet| {
            sheet.key == super::sheet::SHEET_PARAMETERS
                || super::sheet::normalise_sheet_name(&sheet.name)
                    == super::sheet::SHEET_PARAMETERS
        })
        .flat_map(|sheet| &sheet.rows)
        .collect();
    let explicit_config = rows.iter().find_map(|row| {
        if row.text("key").map(str::trim)
            == Some(super::messaging_config::PARAMETER)
        {
            row.get("value")
        } else {
            None
        }
    });
    let config = explicit_config.or_else(|| {
        base.and_then(|v| v.get(super::messaging_config::PARAMETER))
    });
    for row in rows {
        if row.text("key").map(str::trim) != Some("tenant_package") {
            continue;
        }
        let Some(value) = row.get("value").filter(|v| !v.is_null()) else {
            continue;
        };
        // Raw Blueprints may hold encoded JSON rather than a workbook's typed cell.
        let decoded: serde_json::Value;
        let value = if let Some(json) = value.as_str() {
            decoded = serde_json::from_str(json)
                .map_err(|_| problem().at(&row.origin(Some("value"))))?;
            &decoded
        } else {
            value
        };
        if let Some(accounts) =
            value.get("messagingAccounts").filter(|v| !v.is_null())
        {
            let definitions: PortableMessagingAccounts =
                serde_json::from_value(accounts.clone())
                    .map_err(|_| problem().at(&row.origin(Some("value"))))?;
            validate_event_references(
                Some(&definitions),
                event_config(config)?.as_ref(),
            )
            .map_err(|p| p.at(&row.origin(Some("value"))))?;
        }
    }
    Ok(())
}

/// Validate after workbook and separately authorized profile data are combined.
#[cfg(feature = "election_config_archive")]
pub(super) fn validate_delivery(
    layout: &super::archive::Layout,
) -> Result<(), super::problem::Problem> {
    let mut declarations = layout.auxiliary.iter().filter(|a| a.name == MEMBER);
    let parsed = declarations
        .next()
        .map(|a| parse(&a.bytes))
        .transpose()
        .map_err(|_| problem())?;
    if declarations.next().is_some() {
        return Err(problem());
    }
    let events = layout.importable.iter().filter(|a| {
        a.name.starts_with("export_election_event-")
            && a.name.ends_with(".json")
    });
    let mut found = false;
    for a in events {
        if parsed.is_some() && std::mem::replace(&mut found, true) {
            return Err(problem());
        }
        let Ok(event) = serde_json::from_slice::<serde_json::Value>(&a.bytes)
        else {
            if parsed.is_some() {
                return Err(problem());
            }
            // Other event validators own invalid legacy event members.
            continue;
        };
        let config = event
            .get(super::messaging_config::PARAMETER)
            .filter(|v| !v.is_null());
        if parsed.is_none()
            && !config
                .and_then(|v| v.get("channels"))
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .any(|channel| {
                    channel
                        .get("account_id")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|id| id.starts_with(REF_PREFIX))
                })
        {
            continue;
        }
        validate_event_references(
            parsed.as_ref(),
            event_config(config)?.as_ref(),
        )?;
    }
    if parsed.is_some() && !found {
        return validate_event_references(parsed.as_ref(), None);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strictly_consumes_sender_identities() {
        let good = serde_json::json!({"format":FORMAT,"accounts":[
            {"reference":"whatsapp","name":"Demo WhatsApp","sender":{"provider":"WHATSAPP_CLOUD_API","business_account_id":"123","phone_number_id":"456","display_phone_number":"+631234567890","api_version":"v23.0"}},
            {"reference":"viber","name":"Demo Viber","sender":{"provider":"VIBER_INFOBIP","base_url":"https://abc-1.api.infobip.com","sender":"Demo Voice"}},
            {"reference":"facebook","name":"Demo Facebook","sender":{"provider":"MESSENGER_SEND_API","page_id":"123","api_version":"v23.0"}},
            {"reference":"sms","name":"Demo SMS","sender":{"provider":"AWS_SNS","sender_id":"Sample","region":"us-east-1"}},
            {"reference":"email","name":"Demo Email","sender":{"provider":"AWS_SES","from_address":"demo@example.invalid","region":"us-east-1"}}
        ]});
        let bundle = parse(&serde_json::to_vec(&good).unwrap()).unwrap();
        assert_eq!(bundle.accounts.len(), 5);
        assert_eq!(bundle.accounts[0].reference_key(), "package:whatsapp");
        assert_eq!(
            bundle.accounts[0].account_sender().provider().channel(),
            Some(crate::types::messaging::MessageChannel::WHATSAPP)
        );
        for url in [
            "http://a.api.infobip.com",
            "https://a.api.infobip.com/path",
            "https://a.api.infobip.com:443",
            "https://user@a.api.infobip.com",
            "https://a.api.infobip.com?q=1",
            "https://a.api.infobip.com#x",
            "https://a.api.infobip.com.attacker.test",
            "https://a.api.infobip.com./",
        ] {
            let mut bad = good.clone();
            bad["accounts"][1]["sender"]["base_url"] = url.into();
            assert!(parse(&serde_json::to_vec(&bad).unwrap()).is_err());
        }
        for field in [
            "access_token",
            "credentials",
            "api_base_url",
            "approved_templates",
            "notification_topic_arn",
        ] {
            let mut bad = good.clone();
            bad["accounts"][0]["sender"][field] = "CANARY-NOT-IN-ERROR".into();
            let err = parse(&serde_json::to_vec(&bad).unwrap()).unwrap_err();
            assert!(!err.to_string().contains("CANARY"));
        }
        let mut duplicate = good.clone();
        duplicate["accounts"][1]["reference"] = "whatsapp".into();
        assert!(parse(&serde_json::to_vec(&duplicate).unwrap()).is_err());
        let mut future = good;
        future["format"] = "sequent.tenant-messaging-accounts/2".into();
        assert!(parse(&serde_json::to_vec(&future).unwrap()).is_err());
    }

    #[cfg(feature = "election_config_archive")]
    #[test]
    fn refuses_unresolved_or_wrong_channel_refs_but_leaves_legacy_ids() {
        use super::super::archive::{Artifact, Layout};
        let with_config = |id: &str, channel: &str| {
            Artifact {
            name:"export_election_event-sample.json".into(),
            bytes: serde_json::to_vec(&serde_json::json!({"messaging_config":{"version":1,"channels":[{"channel":channel,"account_id":id}]}})).unwrap(),
        }
        };
        let mut layout = Layout {
            importable: vec![with_config("package:sample", "SMS")],
            archive_name: "test.zip".into(),
            auxiliary: Vec::new(),
        };
        assert!(validate_delivery(&layout).is_err());
        layout.auxiliary.push(Artifact {name:MEMBER.into(), bytes: serde_json::to_vec(&serde_json::json!({"format":FORMAT,"accounts":[{"reference":"sample","name":"Test","sender":{"provider":"AWS_SNS"}}]})).unwrap()});
        assert!(validate_delivery(&layout).is_ok());
        layout.importable[0] = with_config("package:sample", "EMAIL");
        assert!(validate_delivery(&layout).is_err());
        layout.importable[0] =
            with_config("0fae91ff-f627-4a66-994a-3049faba72af", "EMAIL");
        // Legacy remains valid without a now-unreferenced portable bundle.
        assert!(validate_delivery(&layout).is_err());
        layout.auxiliary.clear();
        assert!(validate_delivery(&layout).is_ok());
    }

    #[test]
    fn only_senders_with_delivered_credentials_take_a_hex_commitment_never_keys(
    ) {
        let commitment = "ab".repeat(32);
        let aws = |provider: &str, extra: serde_json::Value| {
            let mut sender = serde_json::json!({"provider": provider, "region": "us-east-1"});
            if provider == "AWS_SES" {
                sender["from_address"] = "demo@example.invalid".into();
            }
            let mut account = serde_json::json!({"reference":"aws","name":"Demo AWS","sender":sender});
            for (k, v) in extra.as_object().unwrap() {
                account[k] = v.clone();
            }
            serde_json::json!({"format":FORMAT,"accounts":[account]})
        };
        for provider in ["AWS_SES", "AWS_SNS"] {
            let absent = parse(
                &serde_json::to_vec(&aws(provider, serde_json::json!({})))
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(absent.accounts[0].credential_commitment, None);
            // Omitted stays omitted: older receivers' signed bytes are unchanged.
            assert!(!serde_json::to_string(&absent)
                .unwrap()
                .contains("credential_commitment"));
            let present = aws(
                provider,
                serde_json::json!({"credential_commitment": commitment}),
            );
            assert!(parse(&serde_json::to_vec(&present).unwrap()).is_ok());
            for bad in [
                "AB".repeat(32),
                "ab".repeat(31),
                "zz".repeat(32),
                String::new(),
            ] {
                let doc = aws(
                    provider,
                    serde_json::json!({"credential_commitment": bad}),
                );
                assert!(parse(&serde_json::to_vec(&doc).unwrap()).is_err());
            }
            // The pair itself has no field in the signed declaration.
            for field in
                ["aws_access_key_id", "aws_secret_access_key", "credentials"]
            {
                let mut extra = serde_json::Map::new();
                extra.insert(field.to_string(), "SYNTHETIC-CANARY".into());
                let doc = aws(provider, serde_json::Value::Object(extra));
                let err =
                    parse(&serde_json::to_vec(&doc).unwrap()).unwrap_err();
                assert!(!err.to_string().contains("CANARY"));
            }
        }
        let meta = serde_json::json!({"format":FORMAT,"accounts":[{"reference":"fb","name":"Demo Facebook",
            "sender":{"provider":"MESSENGER_SEND_API","page_id":"123","api_version":"v23.0"},
            "credential_commitment": commitment}]});
        assert!(parse(&serde_json::to_vec(&meta).unwrap()).is_err());
    }

    /// A package's accounts arrive checked: what its author checked, as
    /// the package names it. Each account is approved for exactly the
    /// template languages the package uses on it, and is ready for them.
    #[test]
    fn a_package_account_arrives_checked_for_the_templates_it_names() {
        use crate::types::messaging::{
            purpose_readiness, CheckSource, EventMessagingConfig,
            MessageChannel, ProviderApproval,
        };
        let bundle = parsed(&serde_json::json!({"format":FORMAT,"accounts":[
            {"reference":"whatsapp","name":"Demo WhatsApp","sender":{"provider":"WHATSAPP_INFOBIP",
                "base_url":"https://abc-1.api.infobip.com","sender_number":"447796344125"}},
            {"reference":"viber","name":"Demo Viber","sender":{"provider":"VIBER_INFOBIP",
                "base_url":"https://abc-1.api.infobip.com","sender":"Demo","content_mode":"APPROVED_TEMPLATES",
                "approved_templates":{"OTP":{"es":"code_es"}}}},
            {"reference":"sms","name":"Demo SMS","sender":{"provider":"AWS_SNS","sender_id":"Sample","region":"us-east-1"}}
        ]}))
        .unwrap();
        let event: EventMessagingConfig = serde_json::from_value(serde_json::json!({
            "version":1,"channels":[
                {"channel":"WHATSAPP","account_id":"package:whatsapp","purposes":["OTP","NOTICE"],
                 "templates":[
                    {"purpose":"OTP","key":"otp","language":"en","provider_language":"en_GB","provider_template":"authentication",
                     "buttons":[{"type":"URL","parameter":"{{code}}"}]},
                    {"purpose":"NOTICE","key":"contactChanged.notice.text","language":"es","provider_template":"contact_changed"}
                 ]},
                {"channel":"VIBER","account_id":"package:viber","purposes":["OTP"]}
            ]
        }))
        .unwrap();
        // A message template's WhatsApp part names its own template.
        let notices = [(MessageChannel::WHATSAPP, "fr".to_string())];
        let languages = bundle.declared_languages(Some(&event), &notices);
        let at = "2026-11-27T10:00:00Z".to_string();

        let whatsapp = &bundle.accounts[0];
        let check = whatsapp.declared_check(&languages, at.clone());
        assert!(check.connected && check.production_access);
        assert_eq!(check.checked_at.as_deref(), Some(at.as_str()));
        assert_eq!(check.reason, None);
        assert_eq!(check.details.source, CheckSource::CONFIGURATION_PACKAGE);
        assert_eq!(
            check.details.identity,
            Some(whatsapp.account_sender().identity())
        );
        assert_eq!(
            check.details.provider_approval,
            Some(ProviderApproval::CONFIRMED)
        );
        assert_eq!(
            check.approved_templates,
            BTreeMap::from([
                (MessagePurpose::OTP, vec!["en".into(), "en_GB".into()]),
                (MessagePurpose::NOTICE, vec!["es".into(), "fr".into()]),
            ])
        );
        let sender = whatsapp.account_sender();
        let capabilities =
            sender.capabilities(MessageChannel::WHATSAPP).unwrap();
        for purpose in [MessagePurpose::OTP, MessagePurpose::NOTICE] {
            let readiness = purpose_readiness(
                &capabilities,
                ProviderApproval::PENDING,
                Some(&sender.identity()),
                &check,
                purpose,
                None,
            );
            assert!(readiness.is_ready(), "{purpose}: {readiness:?}");
        }

        // Viber: the template IDs it declares; never WhatsApp's notices.
        let viber = bundle.accounts[1].declared_check(&languages, at.clone());
        assert_eq!(
            viber.approved_templates,
            BTreeMap::from([(MessagePurpose::OTP, vec!["es".into()])])
        );
        // An account without templates is simply connected.
        let sms = bundle.accounts[2].declared_check(&languages, at);
        assert!(sms.connected && sms.approved_templates.is_empty());
        // Checks stored here say nothing of their source.
        let plain = serde_json::to_value(
            crate::types::messaging::AccountCheck::default(),
        )
        .unwrap();
        assert!(plain["details"].get("source").is_none());
        let stored = serde_json::to_value(&sms).unwrap();
        assert_eq!(stored["details"]["source"], "CONFIGURATION_PACKAGE");
    }

    fn one(sender: serde_json::Value) -> serde_json::Value {
        serde_json::json!({"format":FORMAT,"accounts":[
            {"reference":"infobip","name":"Demo Infobip","sender":sender}
        ]})
    }

    fn parsed(doc: &serde_json::Value) -> Option<PortableMessagingAccounts> {
        parse(&serde_json::to_vec(doc).unwrap()).ok()
    }

    #[test]
    fn whatsapp_through_infobip_is_a_portable_sender() {
        use crate::types::messaging::MessageChannel;
        let sender = serde_json::json!({"provider":"WHATSAPP_INFOBIP",
            "base_url":"https://abc-1.api.infobip.com","sender_number":"447796344125",
            "display_name":"Demo Elections"});
        let bundle = parsed(&one(sender.clone())).unwrap();
        assert_eq!(
            bundle.accounts[0].account_sender(),
            AccountSender::WHATSAPP_INFOBIP {
                base_url: "https://abc-1.api.infobip.com".into(),
                sender_number: "447796344125".into(),
                display_name: Some("Demo Elections".into()),
            }
        );
        assert_eq!(
            bundle.accounts[0].account_sender().provider().channel(),
            Some(MessageChannel::WHATSAPP)
        );
        let mut plus = sender.clone();
        plus["sender_number"] = "+447796344125".into();
        assert!(parsed(&one(plus)).is_some());
        let mut unnamed = sender.clone();
        unnamed.as_object_mut().unwrap().remove("display_name");
        assert!(parsed(&one(unnamed)).is_some());
        for (field, bad) in [
            ("base_url", "https://api.infobip.com.attacker.test"),
            ("base_url", "http://abc-1.api.infobip.com"),
            ("sender_number", "0447796344125"),
            ("sender_number", "44 7796 344125"),
            ("sender_number", "1234"),
            ("display_name", " padded"),
        ] {
            let mut doc = sender.clone();
            doc[field] = bad.into();
            assert!(parsed(&one(doc)).is_none(), "{field}={bad}");
        }
        // Credentials never have a field in the declaration.
        for field in ["api_key", "credentials", "report_password"] {
            let mut doc = sender.clone();
            doc[field] = "CANARY-NOT-IN-ERROR".into();
            let err =
                parse(&serde_json::to_vec(&one(doc)).unwrap()).unwrap_err();
            assert!(!err.to_string().contains("CANARY"));
        }
    }

    #[test]
    fn viber_carries_its_content_mode_and_approved_template_ids() {
        use crate::types::messaging::{MessagePurpose, ViberContentMode};
        let base = serde_json::json!({"provider":"VIBER_INFOBIP",
            "base_url":"https://abc-1.api.infobip.com","sender":"Demo Voice"});
        // Omitted: the receiver's defaults, and the signed bytes of older
        // declarations stay exactly as they were.
        let plain = parsed(&one(base.clone())).unwrap();
        let AccountSender::VIBER_INFOBIP {
            content_mode,
            approved_templates,
            ..
        } = plain.accounts[0].account_sender()
        else {
            panic!("VIBER_INFOBIP");
        };
        assert_eq!(content_mode, ViberContentMode::APPROVED_TEMPLATES);
        assert!(approved_templates.is_empty());
        let printed = serde_json::to_string(&plain).unwrap();
        assert!(
            !printed.contains("content_mode")
                && !printed.contains("approved_templates")
        );

        let mut free = base.clone();
        free["content_mode"] = "FREE_TEXT".into();
        let AccountSender::VIBER_INFOBIP { content_mode, .. } =
            parsed(&one(free)).unwrap().accounts[0].account_sender()
        else {
            panic!("VIBER_INFOBIP");
        };
        assert_eq!(content_mode, ViberContentMode::FREE_TEXT);

        let mut templated = base.clone();
        templated["approved_templates"] = serde_json::json!({
            "OTP": {"en": "otp-en-1", "es": "otp_es.2"},
            "NOTICE": {"en": "notice-en"}
        });
        let AccountSender::VIBER_INFOBIP {
            approved_templates, ..
        } = parsed(&one(templated.clone())).unwrap().accounts[0]
            .account_sender()
        else {
            panic!("VIBER_INFOBIP");
        };
        assert_eq!(approved_templates[&MessagePurpose::OTP]["es"], "otp_es.2");
        assert_eq!(
            approved_templates[&MessagePurpose::NOTICE]["en"],
            "notice-en"
        );
        for bad in [
            serde_json::json!({"OTP": {"en": ""}}),
            serde_json::json!({"OTP": {"en": "has space"}}),
            serde_json::json!({"OTP": {"": "otp"}}),
            serde_json::json!({"OTP": {"english language": "otp"}}),
            serde_json::json!({"OTP": {"en": "x".repeat(129)}}),
            serde_json::json!({"MARKETING": {"en": "otp"}}),
        ] {
            let mut doc = base.clone();
            doc["approved_templates"] = bad.clone();
            assert!(parsed(&one(doc)).is_none(), "{bad}");
        }
        let mut unknown_mode = base;
        unknown_mode["content_mode"] = "SOMETHING".into();
        assert!(parsed(&one(unknown_mode)).is_none());
    }

    #[test]
    fn infobip_senders_take_an_api_key_commitment() {
        let commitment = "cd".repeat(32);
        for sender in [
            serde_json::json!({"provider":"VIBER_INFOBIP",
                "base_url":"https://abc-1.api.infobip.com","sender":"Demo Voice"}),
            serde_json::json!({"provider":"WHATSAPP_INFOBIP",
                "base_url":"https://abc-1.api.infobip.com","sender_number":"447796344125"}),
        ] {
            let mut doc = one(sender);
            let bundle = parsed(&doc).unwrap();
            assert_eq!(
                bundle.accounts[0].sender.delivered_credential(),
                Some(DeliveredCredentialKind::API_KEY)
            );
            doc["accounts"][0]["credential_commitment"] =
                commitment.clone().into();
            assert!(parsed(&doc).is_some());
            doc["accounts"][0]["credential_commitment"] =
                "CD".repeat(32).into();
            assert!(parsed(&doc).is_none());
        }
        let ses = parsed(&serde_json::json!({"format":FORMAT,"accounts":[
            {"reference":"email","name":"Demo Email","sender":{"provider":"AWS_SES","from_address":"demo@example.invalid"}}
        ]}))
        .unwrap();
        assert_eq!(
            ses.accounts[0].sender.delivered_credential(),
            Some(DeliveredCredentialKind::AWS_KEY_PAIR)
        );
        let cloud =
            parsed(&one(serde_json::json!({"provider":"WHATSAPP_CLOUD_API",
            "business_account_id":"123","phone_number_id":"456",
            "display_phone_number":"+631234567890","api_version":"v23.0"})))
            .unwrap();
        assert_eq!(cloud.accounts[0].sender.delivered_credential(), None);
    }

    fn sms() -> serde_json::Value {
        serde_json::json!({"format":FORMAT,"accounts":[
            {"reference":"sms","name":"Demo SMS","sender":{"provider":"AWS_SNS","sender_id":"Example","region":"us-east-1"}}
        ]})
    }

    fn event(channels: serde_json::Value) -> serde_json::Value {
        serde_json::json!({"version":1,"channels": channels})
    }

    fn matching() -> serde_json::Value {
        event(serde_json::json!([
            {"channel":"SMS","account_id":"package:sms"},
            {"channel":"EMAIL","account_id":"b39409d6-3300-4d41-b6cf-b9fce289307e"}
        ]))
    }

    #[test]
    fn declarations_reject_duplicate_names_and_technical_sender_identities() {
        let original = sms();
        assert!(parse(&serde_json::to_vec(&original).unwrap()).is_ok());
        let mut same_name = original.clone();
        same_name["accounts"].as_array_mut().unwrap().push(serde_json::json!({
            "reference":"different",
            "name":"DEMO sms",
            "sender":{"provider":"AWS_SNS","sender_id":"Other","region":"us-east-1"}
        }));
        assert!(parse(&serde_json::to_vec(&same_name).unwrap()).is_err());

        let mut same_sender = original;
        let mut other = same_sender["accounts"][0].clone();
        other["reference"] = "different".into();
        other["name"] = "Different name".into();
        same_sender["accounts"].as_array_mut().unwrap().push(other);
        assert!(parse(&serde_json::to_vec(&same_sender).unwrap()).is_err());
    }

    fn parameters(
        config: Option<serde_json::Value>,
    ) -> super::super::sheet::Sheet {
        use super::super::paths::Cell;
        let document = serde_json::json!({
            "format":"sequent.tenant-package/1", "messagingAccounts":sms()
        });
        let mut rows = vec![
            vec![Cell::text("key"), Cell::text("value")],
            vec![
                Cell::text("tenant_package"),
                Cell::text(document.to_string()),
            ],
        ];
        if let Some(config) = config {
            rows.push(vec![
                Cell::text("messaging_config"),
                Cell::text(config.to_string()),
            ]);
        }
        super::super::sheet::Sheet::from_grid("Parameters", &rows).unwrap()
    }

    #[test]
    fn authored_accounts_require_event_configuration_and_every_declared_reference(
    ) {
        assert!(validate_workbook_sheets(&[parameters(None)], None).is_err());
        let unused = event(
            serde_json::json!([{"channel":"SMS","account_id":"legacy-sms"}]),
        );
        assert!(validate_workbook_sheets(&[parameters(Some(unused))], None)
            .is_err());
        let wrong = event(
            serde_json::json!([{"channel":"EMAIL","account_id":"package:sms"}]),
        );
        assert!(
            validate_workbook_sheets(&[parameters(Some(wrong))], None).is_err()
        );
        assert!(validate_workbook_sheets(
            &[parameters(Some(matching()))],
            None
        )
        .is_ok());
        assert!(validate_workbook_sheets(
            &[parameters(None)],
            Some(&serde_json::json!({"messaging_config": matching()}))
        )
        .is_ok());
    }

    #[test]
    #[cfg(feature = "election_config_archive")]
    fn final_delivery_rejects_unused_or_missing_event_and_preserves_legacy_accounts(
    ) {
        use super::super::archive::{delivery, Artifact, Layout};
        let member = |name: &str, value: serde_json::Value| Artifact {
            name: name.to_string(),
            bytes: serde_json::to_vec(&value).unwrap(),
        };
        let mut layout = Layout {
            importable: vec![],
            archive_name: "test.zip".into(),
            auxiliary: vec![member(MEMBER, sms())],
        };
        assert!(delivery(&layout).is_err());
        let name = "export_election_event-synthetic.json";
        layout.importable.push(member(name, serde_json::json!({})));
        assert!(delivery(&layout).is_err());

        let legacy = event(serde_json::json!([
            {"channel":"SMS","account_id":"b39409d6-3300-4d41-b6cf-b9fce289307e"}
        ]));
        layout.importable[0] =
            member(name, serde_json::json!({"messaging_config":legacy}));
        assert!(delivery(&layout).is_err());

        layout.importable[0] =
            member(name, serde_json::json!({"messaging_config":matching()}));
        assert!(delivery(&layout).is_ok());

        layout.auxiliary.clear();
        assert!(
            delivery(&layout).is_err(),
            "no declaration for a package reference"
        );
        layout.importable[0] =
            member(name, serde_json::json!({"messaging_config":legacy}));
        assert!(
            delivery(&layout).is_ok(),
            "legacy UUID-only packages are unchanged"
        );
    }
}
