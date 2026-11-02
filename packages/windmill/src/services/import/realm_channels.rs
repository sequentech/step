// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The voter channels an exported realm published (`sequent.messaging`)
//! without a messaging configuration, and what an import publishes for them.

use sequent_core::types::messaging::{
    MessageChannel, MessengerPage, PublicMessagingChannels, PublicMessengerPage,
    EVENT_MESSAGING_CONFIG_VERSION,
};
use std::collections::BTreeSet;

/// Prefix of every reason an import gives for not publishing the channels
/// an exported realm published.
pub const REALM_CHANNELS_NOT_IMPORTED: &str =
    "Messaging channels of the exported realm not imported";

/// Prefix of every warning an import gives about the channels of an
/// exported realm it published.
pub const REALM_CHANNELS_PUBLISHED: &str = "Messaging channels of the exported realm published";

/// What an import leaves out of realm-only channels it publishes.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealmChannelsAdjustment {
    /// The source tenant's sender names, imported into another tenant.
    SENDER_NAMES_REMOVED,
    /// A Messenger Page none of the importing tenant's Messenger accounts
    /// sends from: voters are not sent to it.
    MESSENGER_PAGE_REMOVED,
}

/// Whether an import brings an event into the tenant it was exported from.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportTenancy {
    SAME_TENANT,
    /// Another tenant: the source's sender names and Messenger Page are not
    /// this tenant's.
    OTHER_TENANT,
}

/// What an import publishes in the new event's realm for the channels the
/// exported realm published ([`REALM_ATTR_MESSAGING`](sequent_core::types::messaging::REALM_ATTR_MESSAGING)).
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq)]
pub enum ImportedRealmChannels {
    /// The exported realm published none.
    NONE,
    /// The bundle carries a messaging configuration: the channels are
    /// published from it once it validates, never from the exported realm.
    FROM_CONFIGURATION,
    /// No messaging configuration travels, because the event had its
    /// channels only in its realm: they are published again as exported,
    /// but for the adjustments listed.
    REALM_ONLY(PublicMessagingChannels, Vec<RealmChannelsAdjustment>),
    /// Realm-only channels that are not published, with the reason.
    NOT_IMPORTED(String),
}

/// What to publish for an exported realm's channels `published`, after the
/// import's id replacement. `configuration` is the bundle's messaging
/// configuration and `election_ids` the imported event's elections: a
/// realm-only projection is kept only when it reads and each election it
/// restricts or names is one of them. Channels and purposes are kept as
/// exported, an empty choice included; into another tenant without the
/// source's sender names. A Messenger Page is kept only as one of
/// `tenant_pages`, the Pages of the importing tenant's own Messenger
/// accounts, and with that account's public details, never the exported ones.
pub fn imported_realm_channels(
    published: Option<&str>,
    configuration: Option<&serde_json::Value>,
    election_ids: &[String],
    tenancy: ImportTenancy,
    tenant_pages: &[MessengerPage],
) -> ImportedRealmChannels {
    let Some(published) = published.filter(|value| !value.trim().is_empty()) else {
        return ImportedRealmChannels::NONE;
    };
    if configuration.is_some() {
        return ImportedRealmChannels::FROM_CONFIGURATION;
    }
    let projection = match serde_json::from_str::<PublicMessagingChannels>(published) {
        Ok(projection) => projection,
        Err(_) => {
            return ImportedRealmChannels::NOT_IMPORTED(format!(
                "{REALM_CHANNELS_NOT_IMPORTED}: they can't be read"
            ))
        }
    };
    if projection.version != EVENT_MESSAGING_CONFIG_VERSION {
        return ImportedRealmChannels::NOT_IMPORTED(format!(
            "{REALM_CHANNELS_NOT_IMPORTED}: unsupported version {}",
            projection.version
        ));
    }
    let unknown: BTreeSet<&String> = projection
        .election_channels
        .keys()
        .chain(projection.election_labels.keys())
        .filter(|election| !election_ids.contains(election))
        .collect();
    if !unknown.is_empty() {
        return ImportedRealmChannels::NOT_IMPORTED(format!(
            "{REALM_CHANNELS_NOT_IMPORTED}: they name elections the event doesn't have: {}",
            unknown.into_iter().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    let mut projection = projection;
    let mut adjustments = vec![];
    if tenancy == ImportTenancy::OTHER_TENANT
        && projection.channels.iter().any(|c| c.sender_label.is_some())
    {
        for channel in &mut projection.channels {
            channel.sender_label = None;
        }
        adjustments.push(RealmChannelsAdjustment::SENDER_NAMES_REMOVED);
    }
    for channel in &mut projection.channels {
        let Some(exported) = channel.messenger_page.take() else {
            continue;
        };
        let owned = (channel.channel == MessageChannel::MESSENGER)
            .then(|| {
                tenant_pages
                    .iter()
                    .find(|page| page.page_id == exported.page_id)
            })
            .flatten();
        match owned {
            Some(page) => {
                channel.messenger_page = Some(PublicMessengerPage {
                    page_id: page.page_id.clone(),
                    username: page.username.clone(),
                    name: page.name.clone(),
                });
            }
            None => {
                if !adjustments.contains(&RealmChannelsAdjustment::MESSENGER_PAGE_REMOVED) {
                    adjustments.push(RealmChannelsAdjustment::MESSENGER_PAGE_REMOVED);
                }
            }
        }
    }
    ImportedRealmChannels::REALM_ONLY(projection, adjustments)
}

/// The channels `projection` offers for some purpose that are sent only
/// through a messaging configuration: all but email and SMS, which the
/// deployment's configured providers send when an event has none.
pub fn needing_configuration(projection: &PublicMessagingChannels) -> Vec<MessageChannel> {
    projection
        .channels
        .iter()
        .filter(|channel| !channel.purposes.is_empty())
        .map(|channel| channel.channel)
        .filter(|channel| !matches!(channel, MessageChannel::EMAIL | MessageChannel::SMS))
        .fold(vec![], |mut channels, channel| {
            if !channels.contains(&channel) {
                channels.push(channel);
            }
            channels
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::messaging::MessagePurpose;

    /// A realm-only projection as an export carries it: every channel for
    /// codes and notices, no election restricted.
    fn exported_realm_channels(election_channels: &str) -> String {
        let channel = |name: &str| {
            format!(
                r#"{{"channel":"{name}","purposes":["OTP","NOTICE"],"sender_label":"Example Org","messenger_page":null}}"#
            )
        };
        format!(
            r#"{{"version":1,"channels":[{}],"election_labels":{{}},"election_channels":{election_channels}}}"#,
            ["WHATSAPP", "VIBER", "MESSENGER", "SMS", "EMAIL"]
                .map(channel)
                .join(",")
        )
    }

    #[test]
    fn realm_only_channels_are_imported_as_exported() {
        let published = exported_realm_channels("{}");
        let ImportedRealmChannels::REALM_ONLY(projection, _) =
            imported_realm_channels(Some(&published), None, &[], ImportTenancy::SAME_TENANT, &[])
        else {
            panic!("realm-only channels not kept");
        };
        assert_eq!(
            projection
                .channels
                .iter()
                .map(|c| c.channel)
                .collect::<Vec<_>>(),
            vec![
                MessageChannel::WHATSAPP,
                MessageChannel::VIBER,
                MessageChannel::MESSENGER,
                MessageChannel::SMS,
                MessageChannel::EMAIL,
            ]
        );
        assert!(projection.channels.iter().all(|c| c.purposes
            == vec![MessagePurpose::OTP, MessagePurpose::NOTICE]
            && c.sender_label.as_deref() == Some("Example Org")));
    }

    #[test]
    fn a_configuration_in_the_bundle_replaces_the_realm_channels() {
        // Even an empty configuration: its validated projection is published.
        let published = exported_realm_channels("{}");
        let empty = serde_json::json!({"version": 1, "channels": []});
        assert_eq!(
            imported_realm_channels(
                Some(&published),
                Some(&empty),
                &[],
                ImportTenancy::SAME_TENANT,
                &[]
            ),
            ImportedRealmChannels::FROM_CONFIGURATION
        );
    }

    #[test]
    fn a_realm_without_channels_imports_none() {
        assert_eq!(
            imported_realm_channels(None, None, &[], ImportTenancy::SAME_TENANT, &[]),
            ImportedRealmChannels::NONE
        );
        assert_eq!(
            imported_realm_channels(Some(" "), None, &[], ImportTenancy::SAME_TENANT, &[]),
            ImportedRealmChannels::NONE
        );
    }

    #[test]
    fn realm_channels_restricting_imported_elections_are_kept() {
        let election = "33333333-3333-4333-8333-333333333333".to_string();
        let published = exported_realm_channels(&format!(r#"{{"{election}":["SMS"]}}"#));
        let ImportedRealmChannels::REALM_ONLY(projection, _) = imported_realm_channels(
            Some(&published),
            None,
            std::slice::from_ref(&election),
            ImportTenancy::SAME_TENANT,
            &[],
        ) else {
            panic!("restricted realm-only channels not kept");
        };
        assert_eq!(
            projection.election_channels.get(&election),
            Some(&vec![MessageChannel::SMS])
        );
    }

    #[test]
    fn realm_channels_naming_another_election_are_not_imported() {
        let published =
            exported_realm_channels(r#"{"44444444-4444-4444-8444-444444444444":["SMS"]}"#);
        let ImportedRealmChannels::NOT_IMPORTED(reason) = imported_realm_channels(
            Some(&published),
            None,
            &["33333333-3333-4333-8333-333333333333".to_string()],
            ImportTenancy::SAME_TENANT,
            &[],
        ) else {
            panic!("channels of an unknown election kept");
        };
        assert!(reason.starts_with(REALM_CHANNELS_NOT_IMPORTED));
        assert!(reason.contains("44444444-4444-4444-8444-444444444444"));
    }

    #[test]
    fn unreadable_realm_channels_are_not_imported() {
        for published in [
            "not json",
            r#"{"version":1,"channels":[{"channel":"PIGEON","purposes":[]}]}"#,
            &exported_realm_channels("{}").replace(r#""version":1"#, r#""version":2"#),
        ] {
            let outcome = imported_realm_channels(
                Some(published),
                None,
                &[],
                ImportTenancy::SAME_TENANT,
                &[],
            );
            assert!(
                matches!(&outcome, ImportedRealmChannels::NOT_IMPORTED(reason)
                    if reason.starts_with(REALM_CHANNELS_NOT_IMPORTED)),
                "{published}: {outcome:?}"
            );
        }
    }

    /// The source's realm-only projection with a Messenger Page.
    fn exported_with_page(page_id: &str) -> String {
        exported_realm_channels("{}").replace(
            r#"{"channel":"MESSENGER","purposes":["OTP","NOTICE"],"sender_label":"Example Org","messenger_page":null}"#,
            &format!(
                r#"{{"channel":"MESSENGER","purposes":["OTP","NOTICE"],"sender_label":"Example Org","messenger_page":{{"page_id":"{page_id}","username":"example","name":"Example Page"}}}}"#
            ),
        )
    }

    fn tenant_page(page_id: &str) -> MessengerPage {
        MessengerPage {
            page_id: page_id.to_string(),
            username: Some("tenant-page".to_string()),
            name: Some("Tenant Page".to_string()),
        }
    }

    #[test]
    fn a_page_on_another_channel_is_removed() {
        let published = exported_realm_channels("{}").replace(
            r#"{"channel":"VIBER","purposes":["OTP","NOTICE"],"sender_label":"Example Org","messenger_page":null}"#,
            r#"{"channel":"VIBER","purposes":["OTP","NOTICE"],"sender_label":"Example Org","messenger_page":{"page_id":"1001","username":null,"name":null}}"#,
        );
        let ImportedRealmChannels::REALM_ONLY(projection, adjustments) = imported_realm_channels(
            Some(&published),
            None,
            &[],
            ImportTenancy::SAME_TENANT,
            &[tenant_page("1001")],
        ) else {
            panic!("realm-only channels not kept");
        };
        assert!(projection
            .channels
            .iter()
            .all(|c| c.messenger_page.is_none()));
        assert_eq!(
            adjustments,
            vec![RealmChannelsAdjustment::MESSENGER_PAGE_REMOVED]
        );
    }

    fn messenger_page(projection: &PublicMessagingChannels) -> Option<&PublicMessengerPage> {
        projection
            .channels
            .iter()
            .find(|c| c.channel == MessageChannel::MESSENGER)
            .and_then(|c| c.messenger_page.as_ref())
    }

    #[test]
    fn a_messenger_page_the_tenant_sends_from_is_kept() {
        let published = exported_with_page("1001");
        let ImportedRealmChannels::REALM_ONLY(projection, adjustments) = imported_realm_channels(
            Some(&published),
            None,
            &[],
            ImportTenancy::SAME_TENANT,
            &[tenant_page("1001")],
        ) else {
            panic!("realm-only channels not kept");
        };
        // The tenant's account's details, not the exported ones.
        assert_eq!(
            messenger_page(&projection),
            Some(&PublicMessengerPage {
                page_id: "1001".to_string(),
                username: Some("tenant-page".to_string()),
                name: Some("Tenant Page".to_string()),
            })
        );
        assert!(adjustments.is_empty());
    }

    #[test]
    fn a_messenger_page_of_no_tenant_account_is_removed_but_the_channel_kept() {
        for (tenancy, pages) in [
            (ImportTenancy::SAME_TENANT, vec![]),
            (ImportTenancy::SAME_TENANT, vec![tenant_page("2002")]),
            (ImportTenancy::OTHER_TENANT, vec![]),
        ] {
            let published = exported_with_page("1001");
            let ImportedRealmChannels::REALM_ONLY(projection, adjustments) =
                imported_realm_channels(Some(&published), None, &[], tenancy, &pages)
            else {
                panic!("realm-only channels not kept");
            };
            assert!(
                messenger_page(&projection).is_none(),
                "{tenancy:?} {pages:?}"
            );
            assert!(projection
                .channels
                .iter()
                .any(|c| c.channel == MessageChannel::MESSENGER));
            assert!(adjustments.contains(&RealmChannelsAdjustment::MESSENGER_PAGE_REMOVED));
        }
    }

    #[test]
    fn another_tenant_gets_no_sender_names() {
        let published = exported_realm_channels("{}");
        let ImportedRealmChannels::REALM_ONLY(projection, adjustments) = imported_realm_channels(
            Some(&published),
            None,
            &[],
            ImportTenancy::OTHER_TENANT,
            &[],
        ) else {
            panic!("realm-only channels not kept");
        };
        assert_eq!(projection.channels.len(), 5);
        assert!(projection.channels.iter().all(|c| c.sender_label.is_none()));
        assert_eq!(
            adjustments,
            vec![RealmChannelsAdjustment::SENDER_NAMES_REMOVED]
        );
    }

    #[test]
    fn explicit_empty_and_disabled_choices_are_kept() {
        // No channel at all, and a channel offered for no purpose.
        for published in [
            r#"{"version":1,"channels":[],"election_labels":{},"election_channels":{}}"#,
            r#"{"version":1,"channels":[{"channel":"WHATSAPP","purposes":[],"sender_label":null,"messenger_page":null}],"election_labels":{},"election_channels":{}}"#,
        ] {
            let expected: PublicMessagingChannels = serde_json::from_str(published).unwrap();
            let ImportedRealmChannels::REALM_ONLY(projection, adjustments) =
                imported_realm_channels(
                    Some(published),
                    None,
                    &[],
                    ImportTenancy::SAME_TENANT,
                    &[],
                )
            else {
                panic!("{published}: not kept");
            };
            assert_eq!(projection, expected);
            assert!(adjustments.is_empty());
            assert!(needing_configuration(&projection).is_empty());
        }
    }

    #[test]
    fn channels_other_than_email_and_sms_need_a_configuration() {
        let projection: PublicMessagingChannels =
            serde_json::from_str(&exported_realm_channels("{}")).unwrap();
        assert_eq!(
            needing_configuration(&projection),
            vec![
                MessageChannel::WHATSAPP,
                MessageChannel::VIBER,
                MessageChannel::MESSENGER,
            ]
        );
    }
}
