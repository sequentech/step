// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use sequent_core::types::keycloak::UserArea;
use sequent_core::types::messaging::{
    AccountCheck, AccountSender, MessagingProvider, ProviderApproval, ReadinessPolicy,
};
use serde_json::json;

fn account(id: u128, channel: MessageChannel, name: &str, is_default: bool) -> MessagingAccount {
    MessagingAccount {
        id: Uuid::from_u128(id),
        tenant_id: Uuid::nil(),
        channel,
        provider: MessagingProvider::AWS_SNS,
        name: name.to_string(),
        sender: AccountSender::AWS_SNS {
            sender_id: None,
            origination_number: None,
            region: None,
        },
        credentials: Default::default(),
        limits: Default::default(),
        provider_approval: ProviderApproval::PENDING,
        readiness: ReadinessPolicy::PROVIDER_CHECK,
        status: AccountCheck::default(),
        webhook_key: String::new(),
        is_default,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// WhatsApp (account 1) and SMS for notices; the Post of election "e2"
/// offers SMS only.
fn config() -> EventMessagingConfig {
    serde_json::from_value(json!({
        "version": 1,
        "channels": [
            {"channel": "WHATSAPP", "account_id": Uuid::from_u128(1).to_string(),
             "purposes": ["OTP", "NOTICE"]},
            {"channel": "SMS", "account_id": Uuid::from_u128(9).to_string(),
             "purposes": ["NOTICE"]}
        ],
        "election_channels": {"e2": ["SMS"]}
    }))
    .unwrap()
}

fn voter(email: Option<&str>, attributes: &[(&str, &str)], area: Option<&str>) -> User {
    User {
        email: email.map(str::to_string),
        attributes: Some(
            attributes
                .iter()
                .map(|(k, v)| (k.to_string(), vec![v.to_string()]))
                .collect(),
        ),
        area: area.map(|id| UserArea {
            id: Some(id.to_string()),
            name: None,
        }),
        ..Default::default()
    }
}

#[test]
fn voters_are_counted_on_their_one_channel_and_unoffered_ones_are_flagged() {
    let config = config();
    let elections = HashMap::from([
        ("a1".to_string(), vec!["e1".to_string()]),
        ("a2".to_string(), vec!["e2".to_string()]),
    ]);
    let whatsapp = [
        ("sequent.read-only.message-channel", "WHATSAPP"),
        ("sequent.read-only.message-number", "+34600000001"),
    ];
    let mut tally = VoterTally::default();
    tally.add_user(
        &voter(None, &whatsapp, Some("a1")),
        Some(&config),
        &elections,
    );
    // Their Post offers SMS only: counted on WhatsApp, not offered.
    tally.add_user(
        &voter(None, &whatsapp, Some("a2")),
        Some(&config),
        &elections,
    );
    // A legacy voter: email is their one channel, which the event doesn't offer.
    tally.add_user(
        &voter(Some("v@example.org"), &[], Some("a1")),
        Some(&config),
        &elections,
    );
    tally.add_user(&voter(None, &[], None), Some(&config), &elections);
    assert_eq!(tally.total, 4);
    assert_eq!(
        tally.channels[&MessageChannel::WHATSAPP],
        ChannelTally {
            voters: 2,
            not_offered: 1
        }
    );
    assert_eq!(
        tally.channels[&MessageChannel::EMAIL],
        ChannelTally {
            voters: 1,
            not_offered: 1
        }
    );
    assert_eq!(tally.no_channel, 1);
}

#[test]
fn the_preview_lists_offered_and_used_channels_in_channel_order_with_their_sender() {
    let config = config();
    let mut tally = VoterTally::default();
    tally.add(Some(MessageChannel::EMAIL), false);
    tally.add(Some(MessageChannel::WHATSAPP), true);
    tally.add(None, false);
    assert_eq!(
        preview_channels(&tally, Some(&config)),
        vec![
            MessageChannel::WHATSAPP,
            MessageChannel::SMS,
            MessageChannel::EMAIL
        ]
    );
    // As `dispatch::account_for` picked them: SMS has no account (the
    // deployment's sender sends it).
    let accounts = BTreeMap::from([
        (
            MessageChannel::WHATSAPP,
            ChannelAccount::Account(account(
                1,
                MessageChannel::WHATSAPP,
                "Election office",
                false,
            )),
        ),
        (MessageChannel::SMS, ChannelAccount::Unset),
        (
            MessageChannel::EMAIL,
            ChannelAccount::Account(account(3, MessageChannel::EMAIL, "Office email", true)),
        ),
    ]);
    let status = VoterCountStatus {
        computed_at: None,
        pending: false,
        outcome: None,
    };
    let preview = audience_rows(&tally, Some(&config), &accounts, status.clone());
    assert_eq!(preview.total, 3);
    // Only the WhatsApp voter is handed to a sender.
    assert_eq!(preview.will_send, 1);
    assert_eq!(preview.status, status);
    let rows: Vec<_> = preview
        .rows
        .iter()
        .map(|row| {
            (
                row.channel,
                row.voters,
                row.not_offered,
                row.sends_from,
                row.account_name.clone(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            (
                Some(MessageChannel::WHATSAPP),
                1,
                0,
                Some(SendsFrom::ACCOUNT),
                Some("Election office".to_string())
            ),
            (
                Some(MessageChannel::SMS),
                0,
                0,
                Some(SendsFrom::DEPLOYMENT),
                None
            ),
            // No voter on Email may get it: nothing is sent, so no sender
            // is shown as if it would send.
            (Some(MessageChannel::EMAIL), 1, 1, None, None),
            (None, 1, 0, None, None),
        ]
    );
}

#[test]
fn a_channel_without_account_or_deployment_sender_sends_from_nothing() {
    assert_eq!(sends_from(None, MessageChannel::VIBER), SendsFrom::NONE);
    assert_eq!(
        sends_from(None, MessageChannel::EMAIL),
        SendsFrom::DEPLOYMENT
    );
}

#[test]
fn selected_voters_must_name_one() {
    assert_eq!(
        Audience::of(
            Some(AudienceSelection::SELECTED),
            Some(vec![" ".to_string()])
        ),
        Err(MessageReason::SELECTED_WITHOUT_VOTERS)
    );
    assert_eq!(Audience::of(None, None), Ok(Audience::All));
}

fn count(channel: Option<MessageChannel>, outcome: &str, messages: i64) -> LedgerCount {
    LedgerCount {
        channel,
        outcome: outcome.to_string(),
        messages,
    }
}

#[test]
fn event_stats_count_messages_by_latest_state_with_unresolved_apart() {
    let all = vec![
        count(None, "FAILED", 2),
        count(Some(MessageChannel::SMS), "DELIVERED", 5),
        count(Some(MessageChannel::SMS), "FAILED", 1),
        count(Some(MessageChannel::SMS), "UNKNOWN", 1),
        count(Some(MessageChannel::SMS), "UNRESOLVED", 4),
    ];
    let last_day = vec![
        count(Some(MessageChannel::SMS), "DELIVERED", 2),
        count(Some(MessageChannel::SMS), "UNKNOWN", 1),
    ];
    let stats = event_stats(&all, &last_day);
    assert_eq!(
        stats,
        vec![
            ChannelStats {
                channel: Some(MessageChannel::SMS),
                sent_last_24h: 3,
                delivered: 5,
                failed: 1,
                unknown: 1,
                unresolved: 4,
                ..Default::default()
            },
            ChannelStats {
                channel: None,
                failed: 2,
                ..Default::default()
            },
        ]
    );
}

#[test]
fn months_are_utc_ranges() {
    let (start, end) = month_range("2026-12").unwrap();
    assert_eq!(start.to_rfc3339(), "2026-12-01T00:00:00+00:00");
    assert_eq!(end.to_rfc3339(), "2027-01-01T00:00:00+00:00");
    for wrong in ["2026-13", "2026-1", "26-01", "2026/01", ""] {
        assert_eq!(month_range(wrong), None, "{wrong}");
    }
}

/// The vector Keycloak's `SupportReferenceTest` and the Admin Portal use.
#[test]
fn support_references_match_keycloak_and_read_like_crockford() {
    let id = Uuid::parse_str("7f0c2a9e-1b3d-4c5e-8f60-123456789abc").unwrap();
    assert_eq!(support_reference(&id), "65SR-MSSD");
    let bits = support_reference_bits(&id);
    for text in ["65SR-MSSD", "65sr mssd", " 65SRMSSD "] {
        assert_eq!(parse_support_reference(text), Some(bits), "{text}");
    }
    // I/L read as 1 and O as 0.
    assert_eq!(
        parse_support_reference("ILOO-0000"),
        parse_support_reference("1100-0000")
    );
    for wrong in ["65SR-MSS", "65SR-MSSDX", "65SR-MSSU", "dela cruz"] {
        assert_eq!(parse_support_reference(wrong), None, "{wrong}");
    }
}

#[test]
fn cached_rows_split_by_vote_and_flag_voters_whose_post_does_not_offer_their_channel() {
    let config = config();
    let elections = HashMap::from([
        ("a1".to_string(), vec!["e1".to_string()]),
        ("a2".to_string(), vec!["e2".to_string()]),
    ]);
    let count = |area: &str, channel, voted, voters| VoterChannelCount {
        area_id: Some(area.to_string()),
        channel,
        voted,
        voters,
    };
    let counts = vec![
        count("a1", Some(MessageChannel::WHATSAPP), true, 4),
        count("a1", Some(MessageChannel::WHATSAPP), false, 6),
        // The Post of e2 offers SMS only.
        count("a2", Some(MessageChannel::WHATSAPP), false, 3),
        count("a2", None, true, 2),
    ];
    let rows = voter_count_rows(&counts, Some(&config), &elections);
    let all = tally_of(&rows, VoterCountScope::ALL);
    assert_eq!(all.total, 15);
    assert_eq!(
        all.channels[&MessageChannel::WHATSAPP],
        ChannelTally {
            voters: 13,
            not_offered: 3
        }
    );
    assert_eq!(all.no_channel, 2);
    let voted = tally_of(&rows, VoterCountScope::VOTED);
    assert_eq!(voted.total, 6);
    assert_eq!(voted.voters(Some(MessageChannel::WHATSAPP)), 4);
    let not_voted = tally_of(&rows, VoterCountScope::NOT_VOTED);
    assert_eq!(
        not_voted.channels[&MessageChannel::WHATSAPP],
        ChannelTally {
            voters: 9,
            not_offered: 3
        }
    );
}

#[test]
fn a_recount_is_pending_until_it_finishes_or_is_too_old() {
    let now = Utc::now();
    let asked = RecountRun {
        requested_at: Some(now - Duration::minutes(5)),
        started_at: None,
        finished_at: Some(now - Duration::hours(2)),
        computed_at: Some(now - Duration::hours(2)),
        outcome: Some(RecountOutcome::COUNTED),
        reason: None,
    };
    assert!(asked.pending(now, RECOUNT_PENDING_FOR));
    let finished = RecountRun {
        finished_at: Some(now - Duration::minutes(1)),
        ..asked.clone()
    };
    assert!(!finished.pending(now, RECOUNT_PENDING_FOR));
    // A request whose task was lost doesn't block a new one for ever.
    let lost = RecountRun {
        requested_at: Some(now - Duration::hours(1)),
        ..asked
    };
    assert!(!lost.pending(now, RECOUNT_PENDING_FOR));
    assert_eq!(
        VoterCountStatus::of(Some(&finished), now),
        VoterCountStatus {
            computed_at: finished.computed_at,
            pending: false,
            outcome: Some(RecountOutcome::COUNTED),
        }
    );
}

/// R8-05: an account the event names that was deleted sends nothing (the
/// send refuses it), never through the deployment's email or SMS sender.
#[test]
fn a_deleted_configured_account_is_shown_as_missing_never_as_the_deployment() {
    let config = config();
    let mut tally = VoterTally::default();
    tally.add(Some(MessageChannel::SMS), true);
    tally.add(Some(MessageChannel::SMS), true);
    let deleted = Uuid::from_u128(9);
    let accounts = BTreeMap::from([(
        MessageChannel::SMS,
        ChannelAccount::Missing {
            account_id: deleted.to_string(),
        },
    )]);
    let status = VoterCountStatus {
        computed_at: None,
        pending: false,
        outcome: None,
    };
    let preview = audience_rows(&tally, Some(&config), &accounts, status);
    let sms = preview
        .rows
        .iter()
        .find(|row| row.channel == Some(MessageChannel::SMS))
        .unwrap();
    assert_eq!(sms.sends_from, Some(SendsFrom::ACCOUNT_MISSING));
    assert_eq!(sms.account_id, Some(deleted));
    assert_eq!(sms.account_name, None);
    assert_eq!(sms.will_send(), 0);
    assert_eq!(preview.will_send, 0);
}
