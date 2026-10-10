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
fn recorded_channel_without_destination_has_no_channel() {
    // Never another channel: the voter's channel can't be used, so none is.
    let a = attrs(&[(MESSAGE_CHANNEL_ATTR_NAME, &["WHATSAPP"])]);
    assert_eq!(voter_channel(Some("voter@example.org"), Some(&a)), None);

    let no_page = attrs(&[
        (MESSAGE_CHANNEL_ATTR_NAME, &["MESSENGER"]),
        (MESSENGER_ID_ATTR_NAME, &["psid-1"]),
        (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
    ]);
    assert_eq!(
        voter_channel(Some("voter@example.org"), Some(&no_page)),
        None
    );
}

#[test]
fn recorded_sms_reads_the_mobile_number_without_a_message_number() {
    let a = attrs(&[
        (MESSAGE_CHANNEL_ATTR_NAME, &["SMS"]),
        (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
    ]);
    let found = voter_channel(Some("voter@example.org"), Some(&a)).unwrap();
    assert_eq!(found.channel, MessageChannel::SMS);
    assert_eq!(found.destination, "+15550000003");
    assert_eq!(found.source, VoterChannelSource::RECORDED);
}

#[test]
fn first_release_record_without_destination_has_no_channel() {
    let a = attrs(&[
        (VERIFIED_CHANNELS_ATTR_NAME, &["WHATSAPP"]),
        (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
    ]);
    assert_eq!(voter_channel(Some("voter@example.org"), Some(&a)), None);
}

#[test]
fn empty_verified_channels_is_not_a_first_release_record() {
    let a = attrs(&[(VERIFIED_CHANNELS_ATTR_NAME, &[])]);
    let found = voter_channel(Some("voter@example.org"), Some(&a)).unwrap();
    assert_eq!(found.channel, MessageChannel::EMAIL);
    assert_eq!(found.source, VoterChannelSource::LEGACY);
}

#[test]
fn channel_values_are_read_trimmed_and_in_any_case() {
    // As the authenticator reads them (`MessageChannel.parse`).
    let recorded = attrs(&[
        (MESSAGE_CHANNEL_ATTR_NAME, &["whatsapp"]),
        (MESSAGE_NUMBER_ATTR_NAME, &["+15550000001"]),
    ]);
    let found =
        voter_channel(Some("voter@example.org"), Some(&recorded)).unwrap();
    assert_eq!(found.channel, MessageChannel::WHATSAPP);
    assert_eq!(found.source, VoterChannelSource::RECORDED);

    let padded = attrs(&[
        (MESSAGE_CHANNEL_ATTR_NAME, &[" SMS "]),
        (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
    ]);
    let found =
        voter_channel(Some("voter@example.org"), Some(&padded)).unwrap();
    assert_eq!(found.channel, MessageChannel::SMS);
    assert_eq!(found.source, VoterChannelSource::RECORDED);

    let verified = attrs(&[
        (MESSAGE_CHANNEL_ATTR_NAME, &["Viber"]),
        (VERIFIED_CHANNELS_ATTR_NAME, &["whatsapp", " Viber"]),
        (WHATSAPP_NUMBER_ATTR_NAME, &["+15550000001"]),
        (VIBER_NUMBER_ATTR_NAME, &["+15550000002"]),
    ]);
    let found = voter_channel(None, Some(&verified)).unwrap();
    assert_eq!(found.channel, MessageChannel::VIBER);
    assert_eq!(found.source, VoterChannelSource::VERIFIED_CHANNELS);
}
