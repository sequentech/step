// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

fn attrs(pairs: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
    pairs
        .iter()
        .map(|(name, values)| {
            (
                name.to_string(),
                values.iter().map(|v| v.to_string()).collect(),
            )
        })
        .collect()
}

#[test]
fn recorded_phone_channel() {
    let a = attrs(&[
        (MESSAGE_CHANNEL_ATTR_NAME, &["VIBER"]),
        (MESSAGE_NUMBER_ATTR_NAME, &["+639171234567"]),
        (MOBILE_PHONE_ATTR_NAME, &["+639170000000"]),
    ]);
    let found = voter_channel(Some("voter@example.org"), Some(&a)).unwrap();
    assert_eq!(found.channel, MessageChannel::VIBER);
    assert_eq!(found.destination, "+639171234567");
    assert_eq!(found.source, VoterChannelSource::RECORDED);
}

#[test]
fn recorded_messenger_needs_its_page() {
    let a = attrs(&[
        (MESSAGE_CHANNEL_ATTR_NAME, &["MESSENGER"]),
        (MESSENGER_ID_ATTR_NAME, &["psid-1"]),
        (MESSENGER_PAGE_ATTR_NAME, &["page-1"]),
    ]);
    let found = voter_channel(None, Some(&a)).unwrap();
    assert_eq!(found.channel, MessageChannel::MESSENGER);
    assert_eq!(found.messenger_page.as_deref(), Some("page-1"));

    let no_page = attrs(&[
        (MESSAGE_CHANNEL_ATTR_NAME, &["MESSENGER"]),
        (MESSENGER_ID_ATTR_NAME, &["psid-1"]),
    ]);
    assert_eq!(voter_channel(None, Some(&no_page)), None);
}

#[test]
fn first_release_record_prefers_its_verified_preference() {
    let a = attrs(&[
        (MESSAGE_CHANNEL_ATTR_NAME, &["VIBER"]),
        (VERIFIED_CHANNELS_ATTR_NAME, &["WHATSAPP", "VIBER"]),
        (WHATSAPP_NUMBER_ATTR_NAME, &["+15550000001"]),
        (VIBER_NUMBER_ATTR_NAME, &["+15550000002"]),
    ]);
    let found = voter_channel(Some("voter@example.org"), Some(&a)).unwrap();
    assert_eq!(found.channel, MessageChannel::VIBER);
    assert_eq!(found.destination, "+15550000002");
    assert_eq!(found.source, VoterChannelSource::VERIFIED_CHANNELS);
}

#[test]
fn first_release_record_ignores_an_unverified_preference() {
    // The notice preference was EMAIL, but only WhatsApp was verified.
    let a = attrs(&[
        (MESSAGE_CHANNEL_ATTR_NAME, &["EMAIL"]),
        (VERIFIED_CHANNELS_ATTR_NAME, &["SMS", "WHATSAPP"]),
        (WHATSAPP_NUMBER_ATTR_NAME, &["+15550000001"]),
        (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
    ]);
    let found = voter_channel(Some("voter@example.org"), Some(&a)).unwrap();
    assert_eq!(found.channel, MessageChannel::WHATSAPP);
}

#[test]
fn legacy_email_then_mobile() {
    let mobile = attrs(&[(MOBILE_PHONE_ATTR_NAME, &["+15550000003"])]);
    let found =
        voter_channel(Some("voter@example.org"), Some(&mobile)).unwrap();
    assert_eq!(found.channel, MessageChannel::EMAIL);
    assert_eq!(found.source, VoterChannelSource::LEGACY);

    let found = voter_channel(Some(""), Some(&mobile)).unwrap();
    assert_eq!(found.channel, MessageChannel::SMS);
    assert_eq!(found.destination, "+15550000003");

    assert_eq!(voter_channel(None, None), None);
}

#[test]
fn recorded_channel_without_destination_is_legacy() {
    let a = attrs(&[(MESSAGE_CHANNEL_ATTR_NAME, &["WHATSAPP"])]);
    let found = voter_channel(Some("voter@example.org"), Some(&a)).unwrap();
    assert_eq!(found.channel, MessageChannel::EMAIL);
    assert_eq!(found.source, VoterChannelSource::LEGACY);
}
