// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The one channel a voter is reached on, read from their Keycloak record.
//!
//! Mirrored in Java by `VoterChannel` (message-otp-authenticator); both
//! must give the same answer for the same record. Keycloak writes the
//! channel attributes only after the voter verified a code on it.

use crate::types::keycloak::{
    MESSAGE_CHANNEL_ATTR_NAME, MESSAGE_NUMBER_ATTR_NAME,
    MESSENGER_ID_ATTR_NAME, MESSENGER_PAGE_ATTR_NAME, MOBILE_PHONE_ATTR_NAME,
    VERIFIED_CHANNELS_ATTR_NAME, VIBER_NUMBER_ATTR_NAME,
    WHATSAPP_NUMBER_ATTR_NAME,
};
use crate::types::messaging::MessageChannel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use strum_macros::{Display, EnumString};

/// Channel order everywhere: WhatsApp, Viber, Messenger, SMS, Email.
pub const CHANNEL_ORDER: [MessageChannel; 5] = [
    MessageChannel::WHATSAPP,
    MessageChannel::VIBER,
    MessageChannel::MESSENGER,
    MessageChannel::SMS,
    MessageChannel::EMAIL,
];

/// Which shape of record the channel was read from. Logged with every
/// outcome, and used by the realm migration that rewrites the second shape
/// into the first.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
)]
pub enum VoterChannelSource {
    /// `message-channel` with its destination (`message-number`,
    /// `messenger-id` and `messenger-page`, or the email).
    RECORDED,
    /// A record from the first messaging release: `verified-channels`, with
    /// `whatsapp-number` or `viber-number`.
    VERIFIED_CHANNELS,
    /// A voter enrolled before messaging: the email, else the mobile number.
    LEGACY,
}

/// A voter's one channel and where it reaches them.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct VoterChannel {
    pub channel: MessageChannel,
    /// E.164 number, email address or Page-scoped ID.
    pub destination: String,
    /// The Page the Page-scoped ID belongs to; Messenger only.
    pub messenger_page: Option<String>,
    pub source: VoterChannelSource,
}

/// A channel as a record holds it: trimmed and in any case, as the
/// authenticator reads it (`MessageChannel.parse`). Anything else is no
/// channel.
pub fn parse_channel_value(value: &str) -> Option<MessageChannel> {
    MessageChannel::from_str(&value.trim().to_ascii_uppercase()).ok()
}

fn first<'a>(
    attributes: Option<&'a HashMap<String, Vec<String>>>,
    name: &str,
) -> Option<&'a str> {
    attributes?
        .get(name)?
        .first()
        .map(String::as_str)
        .filter(|value| !value.is_empty())
}

/// The destination of `channel` in a recorded (ticket-shaped) record.
fn recorded_destination(
    channel: MessageChannel,
    email: Option<&str>,
    attributes: Option<&HashMap<String, Vec<String>>>,
) -> Option<(String, Option<String>)> {
    match channel {
        MessageChannel::WHATSAPP | MessageChannel::VIBER => {
            first(attributes, MESSAGE_NUMBER_ATTR_NAME)
                .map(|n| (n.to_string(), None))
        }
        MessageChannel::SMS => first(attributes, MESSAGE_NUMBER_ATTR_NAME)
            .or_else(|| first(attributes, MOBILE_PHONE_ATTR_NAME))
            .map(|n| (n.to_string(), None)),
        MessageChannel::MESSENGER => messenger(attributes),
        MessageChannel::EMAIL => email.map(|e| (e.to_string(), None)),
    }
}

/// The destination of `channel` in a first-release record.
fn verified_destination(
    channel: MessageChannel,
    email: Option<&str>,
    attributes: Option<&HashMap<String, Vec<String>>>,
) -> Option<(String, Option<String>)> {
    let number = |name| first(attributes, name).map(|n| (n.to_string(), None));
    match channel {
        MessageChannel::WHATSAPP => number(WHATSAPP_NUMBER_ATTR_NAME),
        MessageChannel::VIBER => number(VIBER_NUMBER_ATTR_NAME),
        MessageChannel::SMS => number(MOBILE_PHONE_ATTR_NAME),
        MessageChannel::MESSENGER => messenger(attributes),
        MessageChannel::EMAIL => email.map(|e| (e.to_string(), None)),
    }
}

fn messenger(
    attributes: Option<&HashMap<String, Vec<String>>>,
) -> Option<(String, Option<String>)> {
    let id = first(attributes, MESSENGER_ID_ATTR_NAME)?;
    let page = first(attributes, MESSENGER_PAGE_ATTR_NAME)?;
    Some((id.to_string(), Some(page.to_string())))
}

/// The voter's one channel, or `None` when the record has none that can be
/// used. Never another channel than the record's: a recorded channel whose
/// destination is missing gives `None`, not the email.
///
/// 1. A first-release record (`verified-channels` present and not empty):
///    its `message-channel` if verified, else the first verified channel in
///    [`CHANNEL_ORDER`], each with that release's destination attribute.
///    The realm migration removes `verified-channels` when it rewrites the
///    record, so a rewritten record never takes this path.
/// 2. `message-channel` with its recorded destination (SMS also reads the
///    mobile number when there's no `message-number`: the same channel).
/// 3. Legacy, with neither: the email, else the mobile number. This defines
///    the voter's one channel; it is not a fallback.
pub fn voter_channel(
    email: Option<&str>,
    attributes: Option<&HashMap<String, Vec<String>>>,
) -> Option<VoterChannel> {
    let email = email.filter(|e| !e.is_empty());
    let recorded = first(attributes, MESSAGE_CHANNEL_ATTR_NAME)
        .and_then(parse_channel_value);
    let build = |channel, (destination, messenger_page), source| VoterChannel {
        channel,
        destination,
        messenger_page,
        source,
    };

    if let Some(verified) = attributes
        .and_then(|a| a.get(VERIFIED_CHANNELS_ATTR_NAME))
        .filter(|values| !values.is_empty())
    {
        let verified: Vec<MessageChannel> = verified
            .iter()
            .filter_map(|value| parse_channel_value(value))
            .collect();
        let mut candidates = recorded
            .filter(|channel| verified.contains(channel))
            .into_iter()
            .chain(CHANNEL_ORDER.into_iter().filter(|c| verified.contains(c)));
        return candidates.find_map(|channel| {
            verified_destination(channel, email, attributes).map(|found| {
                build(channel, found, VoterChannelSource::VERIFIED_CHANNELS)
            })
        });
    }
    if let Some(channel) = recorded {
        return recorded_destination(channel, email, attributes)
            .map(|found| build(channel, found, VoterChannelSource::RECORDED));
    }

    if let Some(email) = email {
        return Some(build(
            MessageChannel::EMAIL,
            (email.to_string(), None),
            VoterChannelSource::LEGACY,
        ));
    }
    first(attributes, MOBILE_PHONE_ATTR_NAME).map(|mobile| {
        build(
            MessageChannel::SMS,
            (mobile.to_string(), None),
            VoterChannelSource::LEGACY,
        )
    })
}

#[cfg(test)]
#[path = "voter_channel_tests.rs"]
mod voter_channel_tests;
