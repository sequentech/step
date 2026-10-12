// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The messaging insights' queries against the real schema: monthly
//! volumes and the event tab's counts per message, finding an application
//! by its support reference, and counting an event realm's voters by
//! channel in SQL, held equal to `voter_channel` on the same users.

#[path = "support/schema.rs"]
mod schema;

use chrono::{Duration, TimeZone, Utc};
use deadpool_postgres::Transaction;
use sequent_core::types::messaging::{MessageAttemptState, MessageChannel, UNRESOLVED_AFTER_HOURS};
use uuid::Uuid;
use windmill::postgres::messaging::count_event_messages;
use windmill::postgres::messaging_insights::{
    find_applications_by_support_reference, monthly_volumes, MonthlyVolume,
};
use windmill::services::messaging::insights::{
    event_stats, month_range, parse_support_reference, sort_volumes, support_reference,
    ChannelStats, VoterTally,
};

/// A fixed v4 UUID per test (`seed`) and call (`n`).
fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

struct World {
    tenant: Uuid,
    event: Uuid,
    other_event: Uuid,
    account: Uuid,
}

async fn world(tx: &Transaction<'_>, seed: u32) -> World {
    let world = World {
        tenant: id(seed, 1),
        event: id(seed, 2),
        other_event: id(seed, 3),
        account: id(seed, 4),
    };
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&world.tenant, &format!("tenant-{}", world.tenant)],
    )
    .await
    .unwrap();
    for event in [world.event, world.other_event] {
        tx.execute(
            "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
             VALUES ($1, $2, 'RSA256')",
            &[&event, &world.tenant],
        )
        .await
        .unwrap();
    }
    tx.execute(
        "INSERT INTO sequent_backend.messaging_account
            (id, tenant_id, channel, provider, name, webhook_key)
         VALUES ($1, $2, 'WHATSAPP', 'WHATSAPP_CLOUD_API', 'Election office', $3)",
        &[&world.account, &world.tenant, &format!("{:064x}", seed)],
    )
    .await
    .unwrap();
    world
}

/// Another tenant, with an event and an account of its own.
async fn world_of_tenant(tx: &Transaction<'_>, seed: u32) -> World {
    world(tx, seed).await
}

/// One attempt of a message, written as the ledger writes it.
struct Attempt<'a> {
    event: Uuid,
    key: &'a str,
    attempt: i32,
    channel: Option<&'a str>,
    account: Option<Uuid>,
    country: Option<&'a str>,
    state: &'a str,
    billing_category: Option<&'a str>,
    billable: Option<bool>,
    at: chrono::DateTime<Utc>,
    /// Handed to the provider at `at`.
    dispatched: bool,
    provider_message_id: Option<&'a str>,
}

async fn record(tx: &Transaction<'_>, tenant: &Uuid, attempt: Attempt<'_>) {
    tx.execute(
        "INSERT INTO sequent_backend.message
            (tenant_id, election_event_id, account_id, channel, direction, purpose,
             masked_destination, destination_digest, destination_country, logical_key,
             attempt, state, billing_category, billable, created_at, dispatched_at,
             provider_message_id)
         VALUES ($1, $2, $3, $4, 'OUTBOUND', 'NOTICE', '-', '-', $5, $6, $7, $8, $9, $10, $11,
                 $12, $13)",
        &[
            tenant,
            &attempt.event,
            &attempt.account,
            &attempt.channel,
            &attempt.country,
            &attempt.key,
            &attempt.attempt,
            &attempt.state,
            &attempt.billing_category,
            &attempt.billable,
            &attempt.at,
            &attempt.dispatched.then_some(attempt.at),
            &attempt.provider_message_id,
        ],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn monthly_volumes_count_each_attempt_handed_to_a_provider_in_its_month() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x4a01).await;
    let october = Utc.with_ymd_and_hms(2026, 10, 5, 12, 0, 0).unwrap();
    let base = Attempt {
        event: world.event,
        key: "",
        attempt: 1,
        channel: Some("WHATSAPP"),
        account: Some(world.account),
        country: Some("SA"),
        state: "DELIVERED",
        billing_category: Some("UTILITY"),
        billable: Some(true),
        at: october,
        dispatched: true,
        provider_message_id: None,
    };
    let tenant = world.tenant;
    // Handed to the provider, failed there, retried and delivered: the
    // provider bills both submissions, so both count.
    record(
        &tx,
        &tenant,
        Attempt {
            key: "send-1:a",
            state: "FAILED",
            ..base
        },
    )
    .await;
    record(
        &tx,
        &tenant,
        Attempt {
            key: "send-1:a",
            attempt: 2,
            ..base
        },
    )
    .await;
    record(
        &tx,
        &tenant,
        Attempt {
            key: "send-1:b",
            ..base
        },
    )
    .await;
    // A row from before `dispatched_at`: the provider's id says it was sent.
    record(
        &tx,
        &tenant,
        Attempt {
            key: "send-1:k",
            state: "UNKNOWN",
            dispatched: false,
            provider_message_id: Some("wamid.k"),
            ..base
        },
    )
    .await;
    // Another country, and the other event: rows of their own.
    record(
        &tx,
        &tenant,
        Attempt {
            key: "send-1:c",
            country: Some("PH"),
            ..base
        },
    )
    .await;
    record(
        &tx,
        &tenant,
        Attempt {
            key: "send-1:d",
            event: world.other_event,
            ..base
        },
    )
    .await;
    // Email through the deployment's sender, from before `dispatched_at`.
    record(
        &tx,
        &tenant,
        Attempt {
            key: "send-1:e",
            channel: Some("EMAIL"),
            account: None,
            country: None,
            state: "ACCEPTED",
            billing_category: None,
            billable: None,
            dispatched: false,
            ..base
        },
    )
    .await;
    // Never handed to a provider: not counted. No contact channel; refused
    // (channel not offered); capacity busy, with the expected category;
    // a stale QUEUED the sweep failed.
    for (key, channel, category) in [
        ("send-1:f", None, None),
        ("send-1:h", Some("WHATSAPP"), None),
        ("send-1:i", Some("WHATSAPP"), Some("UTILITY")),
        ("send-1:j", Some("WHATSAPP"), Some("UTILITY")),
    ] {
        record(
            &tx,
            &tenant,
            Attempt {
                key,
                channel,
                state: "FAILED",
                billing_category: category,
                billable: None,
                dispatched: false,
                ..base
            },
        )
        .await;
    }
    // September: another month.
    record(
        &tx,
        &tenant,
        Attempt {
            key: "send-1:g",
            at: october - Duration::days(30),
            ..base
        },
    )
    .await;
    // A message whose retry crosses the month: each attempt in its month.
    let last_minute = Utc.with_ymd_and_hms(2026, 10, 31, 23, 59, 0).unwrap();
    record(
        &tx,
        &tenant,
        Attempt {
            key: "send-1:m",
            state: "FAILED",
            at: last_minute,
            ..base
        },
    )
    .await;
    record(
        &tx,
        &tenant,
        Attempt {
            key: "send-1:m",
            attempt: 2,
            at: last_minute + Duration::minutes(1),
            ..base
        },
    )
    .await;
    // Another tenant's attempt in the same month.
    let other = world_of_tenant(&tx, 0x4a11).await;
    record(
        &tx,
        &other.tenant,
        Attempt {
            event: other.event,
            account: Some(other.account),
            key: "x",
            ..base
        },
    )
    .await;

    let (from, to) = month_range("2026-10").unwrap();
    let mut volumes = monthly_volumes(&tx, &tenant, from, to, Some(&world.event))
        .await
        .unwrap();
    sort_volumes(&mut volumes);
    let whatsapp = |country: &str, messages| MonthlyVolume {
        channel: MessageChannel::WHATSAPP,
        account_id: Some(world.account),
        account_name: Some("Election office".to_string()),
        destination_country: Some(country.to_string()),
        billing_category: Some("UTILITY".to_string()),
        billable: Some(true),
        messages,
    };
    assert_eq!(
        volumes,
        vec![
            whatsapp("PH", 1),
            // a (2 attempts), b, k, and m's October attempt.
            whatsapp("SA", 5),
            MonthlyVolume {
                channel: MessageChannel::EMAIL,
                account_id: None,
                account_name: None,
                destination_country: None,
                billing_category: None,
                billable: None,
                messages: 1,
            },
        ]
    );
    // November holds m's retry.
    let (from, to) = month_range("2026-11").unwrap();
    assert_eq!(
        monthly_volumes(&tx, &tenant, from, to, Some(&world.event))
            .await
            .unwrap(),
        vec![whatsapp("SA", 1)]
    );
    // The whole tenant: the other event's attempt joins Saudi Arabia; the
    // other tenant's never does.
    let (from, to) = month_range("2026-10").unwrap();
    let tenant_wide = monthly_volumes(&tx, &tenant, from, to, None).await.unwrap();
    assert_eq!(
        tenant_wide
            .iter()
            .filter(|row| row.destination_country.as_deref() == Some("SA"))
            .map(|row| row.messages)
            .sum::<i64>(),
        6
    );
    assert_eq!(
        monthly_volumes(&tx, &other.tenant, from, to, None)
            .await
            .unwrap()
            .iter()
            .map(|row| row.messages)
            .sum::<i64>(),
        1
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn event_counts_agree_per_message_with_unresolved_apart() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x4a02).await;
    let now = Utc::now();
    let base = Attempt {
        event: world.event,
        key: "",
        attempt: 1,
        channel: Some("SMS"),
        account: None,
        country: Some("ES"),
        state: "DELIVERED",
        billing_category: None,
        billable: None,
        at: now - Duration::hours(2),
        dispatched: true,
        provider_message_id: None,
    };
    record(
        &tx,
        &world.tenant,
        Attempt {
            key: "k:a",
            state: "FAILED",
            ..base
        },
    )
    .await;
    record(
        &tx,
        &world.tenant,
        Attempt {
            key: "k:a",
            attempt: 2,
            ..base
        },
    )
    .await;
    record(
        &tx,
        &world.tenant,
        Attempt {
            key: "k:b",
            state: "FAILED",
            ..base
        },
    )
    .await;
    record(
        &tx,
        &world.tenant,
        Attempt {
            key: "k:c",
            state: "UNKNOWN",
            at: now - Duration::hours(30),
            ..base
        },
    )
    .await;
    record(
        &tx,
        &world.tenant,
        Attempt {
            key: "k:d",
            channel: None,
            country: None,
            state: "FAILED",
            ..base
        },
    )
    .await;
    let unresolved_before = now - Duration::hours(UNRESOLVED_AFTER_HOURS);
    let all = count_event_messages(&tx, &world.tenant, &world.event, None, unresolved_before)
        .await
        .unwrap();
    let last_day = count_event_messages(
        &tx,
        &world.tenant,
        &world.event,
        Some(now - Duration::hours(24)),
        unresolved_before,
    )
    .await
    .unwrap();
    assert_eq!(
        event_stats(&all, &last_day),
        vec![
            ChannelStats {
                channel: Some(MessageChannel::SMS),
                sent_last_24h: 2,
                delivered: 1,
                failed: 1,
                unresolved: 1,
                ..Default::default()
            },
            ChannelStats {
                channel: None,
                sent_last_24h: 1,
                failed: 1,
                ..Default::default()
            },
        ]
    );
    // The Logs filter's FAILED selection is the same message.
    let failed = windmill::postgres::messaging::latest_attempt_ids(
        &tx,
        &world.tenant,
        &world.event,
        windmill::postgres::messaging::ChannelFilter::Channel(MessageChannel::SMS),
        Some(&MessageAttemptState::FAILED.to_string()),
        unresolved_before,
    )
    .await
    .unwrap();
    assert_eq!(failed.len(), 1);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn an_application_is_found_by_its_support_reference_within_the_queue_scope() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x4a03).await;
    // The vector Keycloak's SupportReferenceTest uses.
    let found = Uuid::parse_str("7f0c2a9e-1b3d-4c5e-8f60-123456789abc").unwrap();
    let other = Uuid::parse_str("7f0c2a9e-1b3d-4c5e-8f60-123456789abd").unwrap();
    for (application, event, label) in [
        (found, world.event, Some("post-a")),
        (other, world.event, None),
    ] {
        tx.execute(
            "INSERT INTO sequent_backend.applications
                (id, tenant_id, election_event_id, area_id, applicant_id, status,
                 verification_type, applicant_data, permission_label)
             VALUES ($1, $2, $3, $4, 'voter', 'PENDING', 'MANUAL', '{}', $5)",
            &[&application, &world.tenant, &event, &Uuid::new_v4(), &label],
        )
        .await
        .unwrap();
    }
    assert_eq!(support_reference(&found), "65SR-MSSD");
    let bits = parse_support_reference("65sr mssd").unwrap();
    let search = |event: Uuid, label: Option<&'static str>| {
        let tx = &tx;
        let tenant = world.tenant;
        async move {
            find_applications_by_support_reference(tx, &tenant, &event, bits, label)
                .await
                .unwrap()
        }
    };
    assert_eq!(search(world.event, None).await, vec![found]);
    assert_eq!(search(world.event, Some("post-a")).await, vec![found]);
    assert!(search(world.event, Some("post-b")).await.is_empty());
    assert!(search(world.other_event, None).await.is_empty());
    // Another tenant never finds this tenant's application, even by its
    // event's id.
    let elsewhere = world_of_tenant(&tx, 0x4a13).await;
    assert!(find_applications_by_support_reference(
        &tx,
        &elsewhere.tenant,
        &world.event,
        bits,
        None
    )
    .await
    .unwrap()
    .is_empty());
    tx.rollback().await.unwrap();
}

// --- Voters by channel, in one SQL aggregate on the Keycloak database ---

use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::keycloak::User;
use sequent_core::types::keycloak::{
    AREA_ID_ATTR_NAME, MESSAGE_CHANNEL_ATTR_NAME, MESSAGE_NUMBER_ATTR_NAME, MESSENGER_ID_ATTR_NAME,
    MESSENGER_PAGE_ATTR_NAME, MOBILE_PHONE_ATTR_NAME, VERIFIED_CHANNELS_ATTR_NAME,
    VIBER_NUMBER_ATTR_NAME, VOTED_CHANNEL, WHATSAPP_NUMBER_ATTR_NAME,
};
use sequent_core::types::messaging::EventMessagingConfig;
use sequent_core::types::voter_channel::{voter_channel, CHANNEL_ORDER};
use std::collections::BTreeMap;
use windmill::postgres::area::get_elections_by_area;
use windmill::postgres::messaging_insights::{
    count_voter_channels, events_due_for_recount, finish_recount, is_statement_timeout,
    recount_run, replace_voter_counts, request_recount, set_statement_timeout, start_recount,
    voter_counts, CachedVoterCount, RecountOutcome, VoterCountScope, Votes,
};
use windmill::services::external::utils::voted_via_not_internet_channel;
use windmill::services::messaging::insights::{
    cached_voters, count_event_voters, count_selected_voters, tally_of, Audience, VoterCountStatus,
    RECOUNT_PENDING_FOR, RECOUNT_TIMEOUT,
};
use windmill::services::users::{list_users, list_users_with_vote_info, ListUsersFilter};

/// A Keycloak-shaped realm schema (the columns `list_users` and the count
/// read), created inside the test's transaction and gone with its rollback.
async fn keycloak_schema(tx: &Transaction<'_>, seed: u32) {
    tx.batch_execute(&format!(
        "CREATE SCHEMA keycloak_{seed:x};
         SET LOCAL search_path TO keycloak_{seed:x}, public;
         CREATE TABLE realm (id varchar(36) PRIMARY KEY, name varchar(255) UNIQUE);
         CREATE TABLE user_entity (
             id varchar(36) PRIMARY KEY, realm_id varchar(255) NOT NULL,
             username varchar(255), email varchar(255), email_verified boolean NOT NULL DEFAULT false,
             enabled boolean NOT NULL DEFAULT true, first_name varchar(255),
             last_name varchar(255), created_timestamp bigint,
             service_account_client_link varchar(255));
         -- Ids in insertion order, so an attribute's first value is the
         -- same to the count (by id) and to `list_users` (as stored).
         CREATE SEQUENCE attribute_order;
         CREATE TABLE user_attribute (
             id varchar(36) PRIMARY KEY
                 DEFAULT lpad(nextval('attribute_order')::text, 36, '0'),
             name varchar(255) NOT NULL, value varchar(255), user_id varchar(36) NOT NULL);
         CREATE INDEX ON user_attribute (user_id);"
    ))
    .await
    .unwrap();
}

/// A voter as Keycloak stores them: one `user_attribute` row per value.
struct Record {
    email: Option<&'static str>,
    attributes: Vec<(&'static str, Vec<&'static str>)>,
    /// A disabled voter is still listed, and sent to, by the bulk send.
    enabled: bool,
}

fn voter_record(
    email: Option<&'static str>,
    attributes: &[(&'static str, &[&'static str])],
) -> Record {
    Record {
        email,
        attributes: attributes
            .iter()
            .map(|(name, values)| (*name, values.to_vec()))
            .collect(),
        enabled: true,
    }
}

fn disabled(record: Record) -> Record {
    Record {
        enabled: false,
        ..record
    }
}

/// Every case of `voter_channel_tests.rs`, and the edges of the SQL's
/// reading of attributes.
fn records() -> Vec<Record> {
    let email = Some("voter@example.org");
    vec![
        // recorded_phone_channel
        voter_record(
            email,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["VIBER"]),
                (MESSAGE_NUMBER_ATTR_NAME, &["+639171234567"]),
                (MOBILE_PHONE_ATTR_NAME, &["+639170000000"]),
            ],
        ),
        // recorded_messenger_needs_its_page
        voter_record(
            None,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["MESSENGER"]),
                (MESSENGER_ID_ATTR_NAME, &["psid-1"]),
                (MESSENGER_PAGE_ATTR_NAME, &["page-1"]),
            ],
        ),
        voter_record(
            None,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["MESSENGER"]),
                (MESSENGER_ID_ATTR_NAME, &["psid-1"]),
            ],
        ),
        // first_release_record_prefers_its_verified_preference
        voter_record(
            email,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["VIBER"]),
                (VERIFIED_CHANNELS_ATTR_NAME, &["WHATSAPP", "VIBER"]),
                (WHATSAPP_NUMBER_ATTR_NAME, &["+15550000001"]),
                (VIBER_NUMBER_ATTR_NAME, &["+15550000002"]),
            ],
        ),
        // first_release_record_ignores_an_unverified_preference
        voter_record(
            email,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["EMAIL"]),
                (VERIFIED_CHANNELS_ATTR_NAME, &["SMS", "WHATSAPP"]),
                (WHATSAPP_NUMBER_ATTR_NAME, &["+15550000001"]),
                (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
            ],
        ),
        // legacy_email_then_mobile
        voter_record(email, &[(MOBILE_PHONE_ATTR_NAME, &["+15550000003"])]),
        voter_record(Some(""), &[(MOBILE_PHONE_ATTR_NAME, &["+15550000003"])]),
        voter_record(None, &[]),
        // recorded_channel_without_destination_has_no_channel
        voter_record(email, &[(MESSAGE_CHANNEL_ATTR_NAME, &["WHATSAPP"])]),
        voter_record(
            email,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["MESSENGER"]),
                (MESSENGER_ID_ATTR_NAME, &["psid-1"]),
                (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
            ],
        ),
        // recorded_sms_reads_the_mobile_number_without_a_message_number
        voter_record(
            email,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["SMS"]),
                (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
            ],
        ),
        // first_release_record_without_destination_has_no_channel
        voter_record(
            email,
            &[
                (VERIFIED_CHANNELS_ATTR_NAME, &["WHATSAPP"]),
                (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
            ],
        ),
        // empty_verified_channels_is_not_a_first_release_record: Keycloak
        // stores an empty list as no rows.
        voter_record(email, &[(VERIFIED_CHANNELS_ATTR_NAME, &[])]),
        // Beyond the unit cases: a verified list holding only an empty
        // value is a first-release record with nothing verified.
        voter_record(email, &[(VERIFIED_CHANNELS_ATTR_NAME, &[""])]),
        // An unknown recorded channel is no recorded channel: legacy.
        voter_record(email, &[(MESSAGE_CHANNEL_ATTR_NAME, &["FAX"])]),
        // Empty values are absent values.
        voter_record(
            email,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["WHATSAPP"]),
                (MESSAGE_NUMBER_ATTR_NAME, &[""]),
            ],
        ),
        voter_record(
            None,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["EMAIL"]),
                (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
            ],
        ),
        voter_record(
            None,
            &[
                (VERIFIED_CHANNELS_ATTR_NAME, &["SMS", "EMAIL"]),
                (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
            ],
        ),
        voter_record(
            email,
            &[
                (VERIFIED_CHANNELS_ATTR_NAME, &["MESSENGER", "EMAIL"]),
                (MESSENGER_ID_ATTR_NAME, &["psid-2"]),
                (MESSENGER_PAGE_ATTR_NAME, &["page-2"]),
                (MESSAGE_CHANNEL_ATTR_NAME, &["EMAIL"]),
            ],
        ),
        voter_record(
            None,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["WHATSAPP"]),
                (MESSAGE_NUMBER_ATTR_NAME, &["+15550000004"]),
                (VOTED_CHANNEL, &["Kiosk"]),
            ],
        ),
        voter_record(email, &[(VOTED_CHANNEL, &["Internet"])]),
        // A reset vote ("NONE") is no vote outside the internet channel.
        voter_record(
            email,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["SMS"]),
                (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
                (VOTED_CHANNEL, &["Kiosk", "NONE"]),
            ],
        ),
        voter_record(email, &[(VOTED_CHANNEL, &["none"])]),
        // Channel values trimmed and in any case (R8-07).
        voter_record(
            email,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["whatsapp"]),
                (MESSAGE_NUMBER_ATTR_NAME, &["+15550000005"]),
            ],
        ),
        voter_record(
            email,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &[" SMS "]),
                (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
            ],
        ),
        voter_record(
            email,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["Viber"]),
                (VERIFIED_CHANNELS_ATTR_NAME, &["whatsapp", " Viber"]),
                (WHATSAPP_NUMBER_ATTR_NAME, &["+15550000001"]),
                (VIBER_NUMBER_ATTR_NAME, &["+15550000002"]),
            ],
        ),
        voter_record(
            None,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["   "]),
                (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
            ],
        ),
        // A multi-valued channel reads its first value: an empty one is no
        // recorded channel, whatever follows.
        voter_record(
            email,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["", "SMS"]),
                (MOBILE_PHONE_ATTR_NAME, &["+15550000003"]),
            ],
        ),
        // A disabled voter counts: the bulk send lists them.
        disabled(voter_record(
            email,
            &[
                (MESSAGE_CHANNEL_ATTR_NAME, &["VIBER"]),
                (MESSAGE_NUMBER_ATTR_NAME, &["+15550000006"]),
            ],
        )),
    ]
}

struct Realm {
    tenant: Uuid,
    event: Uuid,
    realm: String,
    open_area: Uuid,
    restricted_area: Uuid,
    /// The event's voters, in insertion order.
    voters: Vec<String>,
    /// Of them, those with a valid ballot.
    voted: Vec<String>,
    config: EventMessagingConfig,
}

/// The event: two Posts (one restricted to SMS for notices), each voter
/// case in each area and with no area, every other voter with a ballot,
/// and users the send never lists (another realm, a service account, an
/// area of another event).
async fn realm_of_every_case(tx: &Transaction<'_>, seed: u32) -> Realm {
    keycloak_schema(tx, seed).await;
    let base = world(tx, seed).await;
    let (open, restricted) = (id(seed, 10), id(seed, 11));
    let (open_area, restricted_area) = (id(seed, 12), id(seed, 13));
    for (election, area, contest) in [
        (open, open_area, id(seed, 14)),
        (restricted, restricted_area, id(seed, 15)),
    ] {
        tx.execute(
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&election, &base.tenant, &base.event],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
             VALUES ($1, $2, $3, 'Post')",
            &[&area, &base.tenant, &base.event],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
             VALUES ($1, $2, $3, $4)",
            &[&contest, &base.tenant, &base.event, &election],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sequent_backend.area_contest
                 (id, tenant_id, election_event_id, area_id, contest_id)
             VALUES ($1, $2, $3, $4, $5)",
            &[&Uuid::new_v4(), &base.tenant, &base.event, &area, &contest],
        )
        .await
        .unwrap();
    }
    let realm = get_event_realm(&base.tenant.to_string(), &base.event.to_string());
    let realm_id = format!("realm-{seed:x}");
    tx.execute(
        "INSERT INTO realm (id, name) VALUES ($1, $2), ('elsewhere', 'another realm')",
        &[&realm_id, &realm],
    )
    .await
    .unwrap();
    let insert_user = |user: String,
                       realm_id: String,
                       email: Option<&'static str>,
                       link: Option<&'static str>,
                       enabled: bool| async move {
        tx.execute(
            "INSERT INTO user_entity
                 (id, realm_id, username, email, service_account_client_link, enabled)
             VALUES ($1, $2, $1, $3, $4, $5)",
            &[&user, &realm_id, &email, &link, &enabled],
        )
        .await
        .unwrap();
    };
    let insert_attribute = |user: String, name: &'static str, value: String| async move {
        tx.execute(
            "INSERT INTO user_attribute (name, value, user_id) VALUES ($1, $2, $3)",
            &[&name, &value, &user],
        )
        .await
        .unwrap();
    };
    let other_event_area = Uuid::new_v4().to_string();
    let areas = [
        Some(open_area.to_string()),
        Some(restricted_area.to_string()),
        None,
    ];
    let mut voters = vec![];
    let mut voted = vec![];
    let mut number = 100;
    for record in records() {
        for area in &areas {
            number += 1;
            let user = id(seed, number).to_string();
            insert_user(
                user.clone(),
                realm_id.clone(),
                record.email,
                None,
                record.enabled,
            )
            .await;
            for (name, values) in &record.attributes {
                for value in values {
                    insert_attribute(user.clone(), *name, value.to_string()).await;
                }
            }
            if let Some(area) = area {
                insert_attribute(user.clone(), AREA_ID_ATTR_NAME, area.clone()).await;
            }
            if number % 2 == 0 {
                let (election, area) = match area {
                    Some(area) if *area == restricted_area.to_string() => {
                        (restricted, restricted_area)
                    }
                    _ => (open, open_area),
                };
                tx.execute(
                    "INSERT INTO sequent_backend.cast_vote
                         (tenant_id, election_event_id, election_id, area_id, voter_id_string, status)
                     VALUES ($1, $2, $3, $4, $5, 'valid')",
                    &[&base.tenant, &base.event, &election, &area, &user],
                )
                .await
                .unwrap();
                voted.push(user.clone());
            } else if number % 5 == 0 {
                // A discarded ballot is no vote.
                tx.execute(
                    "INSERT INTO sequent_backend.cast_vote
                         (tenant_id, election_event_id, election_id, area_id, voter_id_string, status)
                     VALUES ($1, $2, $3, $4, $5, 'discarded')",
                    &[&base.tenant, &base.event, &open, &open_area, &user],
                )
                .await
                .unwrap();
            }
            voters.push(user);
        }
    }
    // Never listed: another realm, a service account, another event's area.
    let email = Some("voter@example.org");
    insert_user(
        id(seed, 900).to_string(),
        "elsewhere".to_string(),
        email,
        None,
        true,
    )
    .await;
    insert_user(
        id(seed, 901).to_string(),
        realm_id.clone(),
        email,
        Some("client"),
        true,
    )
    .await;
    insert_user(
        id(seed, 902).to_string(),
        realm_id.clone(),
        email,
        None,
        true,
    )
    .await;
    insert_attribute(
        id(seed, 902).to_string(),
        AREA_ID_ATTR_NAME,
        other_event_area,
    )
    .await;
    // A voter with two areas of the event counts once, in the first (R8-09).
    let two_areas = id(seed, 903).to_string();
    insert_user(two_areas.clone(), realm_id.clone(), None, None, true).await;
    for (name, value) in [
        (MESSAGE_CHANNEL_ATTR_NAME, "WHATSAPP".to_string()),
        (MESSAGE_NUMBER_ATTR_NAME, "+15550000007".to_string()),
        (AREA_ID_ATTR_NAME, open_area.to_string()),
        (AREA_ID_ATTR_NAME, restricted_area.to_string()),
    ] {
        insert_attribute(two_areas.clone(), name, value).await;
    }
    tx.execute(
        "INSERT INTO sequent_backend.cast_vote
             (tenant_id, election_event_id, election_id, area_id, voter_id_string, status)
         VALUES ($1, $2, $3, $4, $5, 'valid')",
        &[&base.tenant, &base.event, &open, &open_area, &two_areas],
    )
    .await
    .unwrap();
    voters.push(two_areas.clone());
    voted.push(two_areas);

    let any = Uuid::new_v4().to_string();
    let channels: Vec<serde_json::Value> = CHANNEL_ORDER
        .into_iter()
        .map(|channel| {
            serde_json::json!({
                "channel": channel, "account_id": any, "purposes": ["NOTICE"]
            })
        })
        .collect();
    let config: EventMessagingConfig = serde_json::from_value(serde_json::json!({
        "version": 1,
        "channels": channels,
        "election_channels": {restricted.to_string(): ["SMS"]}
    }))
    .unwrap();
    Realm {
        tenant: base.tenant,
        event: base.event,
        realm,
        open_area,
        restricted_area,
        voters,
        voted,
        config,
    }
}

/// The count as the bulk send reads voters: `list_users` (with
/// `votes_info` for the voted filters) in pages, `voter_channel` on each.
/// `list_users` lists a voter with two of the event's areas once per area;
/// each voter is counted once.
async fn count_by_listing(tx: &Transaction<'_>, realm: &Realm, audience: &Audience) -> VoterTally {
    let (tenant, event) = (realm.tenant.to_string(), realm.event.to_string());
    let elections_by_area = get_elections_by_area(tx, &tenant, &event).await.unwrap();
    let user_ids = match audience {
        Audience::Selected(ids) => Some(ids.clone()),
        _ => None,
    };
    let mut tally = VoterTally::default();
    let mut seen = std::collections::HashSet::new();
    let mut processed = 0;
    loop {
        let filter = ListUsersFilter {
            election_event_id: Some(event.clone()),
            limit: Some(7),
            offset: Some(processed),
            user_ids: user_ids.clone(),
            ..ListUsersFilter::new(&tenant, &realm.realm)
        };
        let (users, total) = match audience {
            Audience::Voted | Audience::NotVoted => {
                list_users_with_vote_info(tx, tx, filter).await.unwrap()
            }
            _ => list_users(tx, tx, filter).await.unwrap(),
        };
        for user in &users {
            if !seen.insert(user.id.clone()) {
                continue;
            }
            let voted = user.votes_info.as_ref().map(|votes| !votes.is_empty());
            let included = match audience {
                Audience::Voted => voted == Some(true),
                Audience::NotVoted => voted == Some(false),
                _ => true,
            };
            if included {
                tally.add_user(user, Some(&realm.config), &elections_by_area);
            }
        }
        processed += users.len() as i32;
        if users.is_empty() || processed >= total {
            break;
        }
    }
    tally
}

/// The event's voters, as `list_users` lists them, each once.
async fn listed_users(tx: &Transaction<'_>, realm: &Realm) -> Vec<User> {
    let (users, _) = list_users(
        tx,
        tx,
        ListUsersFilter {
            election_event_id: Some(realm.event.to_string()),
            limit: Some(1000),
            ..ListUsersFilter::new(&realm.tenant.to_string(), &realm.realm)
        },
    )
    .await
    .unwrap();
    let mut seen = std::collections::HashSet::new();
    users
        .into_iter()
        .filter(|user| seen.insert(user.id.clone()))
        .collect()
}

#[tokio::test]
async fn voters_counted_in_sql_equal_voter_channel_on_the_same_users() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let realm = realm_of_every_case(&tx, 0x4a04).await;
    let rows = count_event_voters(
        &tx,
        &tx,
        &realm.tenant,
        &realm.event,
        Some(&realm.config),
        RECOUNT_TIMEOUT,
    )
    .await
    .unwrap();
    for (audience, scope) in [
        (Audience::All, VoterCountScope::ALL),
        (Audience::Voted, VoterCountScope::VOTED),
        (Audience::NotVoted, VoterCountScope::NOT_VOTED),
    ] {
        let expected = count_by_listing(&tx, &realm, &audience).await;
        assert_eq!(tally_of(&rows, scope), expected, "{audience:?}");
    }
    let selected: Vec<String> = realm
        .voters
        .iter()
        .step_by(3)
        .cloned()
        .chain([
            id(0x4a04, 900).to_string(),
            id(0x4a04, 901).to_string(),
            id(0x4a04, 903).to_string(),
        ])
        .collect();
    let counted = count_selected_voters(
        &tx,
        &tx,
        &realm.tenant,
        &realm.event,
        Some(&realm.config),
        &selected,
        RECOUNT_TIMEOUT,
    )
    .await
    .unwrap()
    .expect("counted within the time limit");
    assert_eq!(
        counted,
        count_by_listing(&tx, &realm, &Audience::Selected(selected)).await
    );
    // Every case is in the realm: each channel, no channel, and voters of
    // the SMS-only Post on other channels (not offered them). The voter
    // with two areas counts once.
    let all = tally_of(&rows, VoterCountScope::ALL);
    assert_eq!(all.total, realm.voters.len() as i64);
    assert_eq!(
        tally_of(&rows, VoterCountScope::VOTED).total
            + tally_of(&rows, VoterCountScope::NOT_VOTED).total,
        all.total
    );
    assert!(all.no_channel > 0);
    for channel in CHANNEL_ORDER {
        assert!(all.voters(Some(channel)) > 0, "{channel}");
    }
    assert!(all.channels[&MessageChannel::WHATSAPP].not_offered > 0);
    assert_eq!(all.channels[&MessageChannel::SMS].not_offered, 0);
    tx.rollback().await.unwrap();
}

/// The aggregate per area, channel and vote, against `voter_channel` on
/// each user `list_users` reads, including a Datafix event's votes outside
/// the internet channel.
#[tokio::test]
async fn the_aggregate_groups_voter_channel_by_area_and_vote() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let realm = realm_of_every_case(&tx, 0x4a05).await;
    let users = listed_users(&tx, &realm).await;
    assert_eq!(users.len(), realm.voters.len());
    let area_ids = vec![
        realm.open_area.to_string(),
        realm.restricted_area.to_string(),
    ];
    let expected = |voted: &dyn Fn(&User) -> bool| {
        let mut counts: BTreeMap<(Option<String>, Option<MessageChannel>, bool), i64> =
            BTreeMap::new();
        for user in &users {
            let channel = voter_channel(user.email.as_deref(), user.attributes.as_ref())
                .map(|found| found.channel);
            let area = user
                .attributes
                .as_ref()
                .and_then(|attributes| attributes.get(AREA_ID_ATTR_NAME))
                .and_then(|values| values.first().cloned());
            *counts.entry((area, channel, voted(user))).or_default() += 1;
        }
        counts
    };
    let counted = |votes: Votes| {
        let (tx, realm, area_ids) = (&tx, &realm, &area_ids);
        async move {
            count_voter_channels(tx, &realm.realm, None, area_ids, &votes)
                .await
                .unwrap()
                .into_iter()
                .map(|count| ((count.area_id, count.channel, count.voted), count.voters))
                .collect::<BTreeMap<_, _>>()
        }
    };
    let voted = |user: &User, datafix: bool| {
        realm.voted.contains(user.id.as_ref().unwrap())
            || (datafix && voted_via_not_internet_channel(user.attributes.as_ref().unwrap()))
    };
    assert_eq!(counted(Votes::default()).await, expected(&|_: &User| false));
    for datafix in [false, true] {
        assert_eq!(
            counted(Votes {
                voter_ids: realm.voted.clone(),
                datafix,
            })
            .await,
            expected(&|user: &User| voted(user, datafix)),
            "datafix {datafix}"
        );
    }
    tx.rollback().await.unwrap();
}

fn cached(
    scope: VoterCountScope,
    area_id: Option<&str>,
    channel: Option<MessageChannel>,
    voters: i64,
    not_offered: i64,
) -> CachedVoterCount {
    CachedVoterCount {
        scope,
        area_id: area_id.map(str::to_string),
        channel,
        voters,
        not_offered,
    }
}

/// The cache holds one event's counts per scope, area and channel (no area
/// and no channel included), and a recount replaces them whole.
#[tokio::test]
async fn voter_counts_are_cached_per_event_and_replaced_by_a_recount() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x4a06).await;
    let at = Utc.with_ymd_and_hms(2026, 10, 1, 8, 0, 0).unwrap();
    let first = vec![
        cached(
            VoterCountScope::ALL,
            Some("a"),
            Some(MessageChannel::SMS),
            3,
            1,
        ),
        cached(VoterCountScope::ALL, None, Some(MessageChannel::SMS), 2, 0),
        cached(VoterCountScope::ALL, None, None, 4, 0),
        cached(VoterCountScope::VOTED, None, None, 1, 0),
    ];
    replace_voter_counts(&tx, &world.tenant, &world.event, &first, at)
        .await
        .unwrap();
    replace_voter_counts(
        &tx,
        &world.tenant,
        &world.other_event,
        &[cached(VoterCountScope::ALL, None, None, 9, 0)],
        at,
    )
    .await
    .unwrap();
    let mut all = voter_counts(&tx, &world.tenant, &world.event, VoterCountScope::ALL)
        .await
        .unwrap();
    all.sort_by_key(|count| (count.area_id.clone(), count.channel));
    assert_eq!(
        all,
        vec![first[2].clone(), first[1].clone(), first[0].clone(),]
    );
    let tally = tally_of(&all, VoterCountScope::ALL);
    assert_eq!(tally.total, 9);
    assert_eq!(tally.channels[&MessageChannel::SMS].voters, 5);
    assert_eq!(tally.channels[&MessageChannel::SMS].not_offered, 1);
    assert_eq!(tally.no_channel, 4);
    // A recount replaces the event's counts, and only its own.
    let second = vec![cached(
        VoterCountScope::NOT_VOTED,
        None,
        Some(MessageChannel::EMAIL),
        7,
        0,
    )];
    replace_voter_counts(&tx, &world.tenant, &world.event, &second, at)
        .await
        .unwrap();
    for scope in [VoterCountScope::ALL, VoterCountScope::VOTED] {
        assert!(voter_counts(&tx, &world.tenant, &world.event, scope)
            .await
            .unwrap()
            .is_empty());
    }
    assert_eq!(
        voter_counts(&tx, &world.tenant, &world.event, VoterCountScope::NOT_VOTED)
            .await
            .unwrap(),
        second
    );
    assert_eq!(
        voter_counts(&tx, &world.tenant, &world.other_event, VoterCountScope::ALL)
            .await
            .unwrap()
            .len(),
        1
    );
    // Rows are unique even where area and channel are NULL.
    assert!(replace_voter_counts(
        &tx,
        &world.tenant,
        &world.event,
        &[
            cached(VoterCountScope::ALL, None, None, 1, 0),
            cached(VoterCountScope::ALL, None, None, 1, 0),
        ],
        at,
    )
    .await
    .is_err());
    tx.rollback().await.unwrap();
}

/// A recount is asked for once until it finishes or is too old; only a
/// counted one moves "Counted at"; beat asks for the events with a
/// messaging configuration counted longest ago, never-counted first.
#[tokio::test]
async fn a_recount_is_asked_for_once_and_beat_picks_the_events_due() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let world = world(&tx, 0x4a07).await;
    let (tenant, event) = (world.tenant, world.event);
    let now = Utc.with_ymd_and_hms(2026, 10, 1, 8, 0, 0).unwrap();
    let minutes = Duration::minutes;
    let status = |at| {
        let tx = &tx;
        async move {
            cached_voters(tx, &tenant, &event, VoterCountScope::ALL, at)
                .await
                .unwrap()
                .1
        }
    };
    assert_eq!(
        status(now).await,
        VoterCountStatus {
            computed_at: None,
            pending: false,
            outcome: None,
        }
    );
    assert!(
        request_recount(&tx, &tenant, &event, now, RECOUNT_PENDING_FOR)
            .await
            .unwrap()
    );
    // A second ask while pending (the Recount button twice, a save) asks
    // nothing more.
    assert!(
        !request_recount(&tx, &tenant, &event, now + minutes(1), RECOUNT_PENDING_FOR)
            .await
            .unwrap()
    );
    assert!(status(now + minutes(1)).await.pending);
    start_recount(&tx, &tenant, &event, now + minutes(2))
        .await
        .unwrap();
    finish_recount(
        &tx,
        &tenant,
        &event,
        now + minutes(3),
        RecountOutcome::COUNTED,
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        status(now + minutes(3)).await,
        VoterCountStatus {
            computed_at: Some(now + minutes(3)),
            pending: false,
            outcome: Some(RecountOutcome::COUNTED),
        }
    );
    // A recount that takes too long keeps the counts shown, and says why.
    assert!(
        request_recount(&tx, &tenant, &event, now + minutes(4), RECOUNT_PENDING_FOR)
            .await
            .unwrap()
    );
    finish_recount(
        &tx,
        &tenant,
        &event,
        now + minutes(5),
        RecountOutcome::TIMED_OUT,
        Some("Counting the voters took longer than 600 seconds"),
    )
    .await
    .unwrap();
    let run = recount_run(&tx, &tenant, &event).await.unwrap().unwrap();
    assert_eq!(run.computed_at, Some(now + minutes(3)));
    assert_eq!(run.outcome, Some(RecountOutcome::TIMED_OUT));
    assert_eq!(
        run.reason.as_deref(),
        Some("Counting the voters took longer than 600 seconds")
    );
    // A recount asked for and lost (its task never ran) may be asked for
    // again once it is too old.
    assert!(
        request_recount(&tx, &tenant, &event, now + minutes(6), RECOUNT_PENDING_FOR)
            .await
            .unwrap()
    );
    let later = now + minutes(6) + RECOUNT_PENDING_FOR + minutes(1);
    assert!(!status(later).await.pending);
    assert!(
        request_recount(&tx, &tenant, &event, later, RECOUNT_PENDING_FOR)
            .await
            .unwrap()
    );

    // Beat: only events with a messaging configuration; never counted
    // first; counted recently or pending left alone.
    let configured = world_of_tenant(&tx, 0x4a17).await;
    for event in [world.event, world.other_event] {
        tx.execute(
            "INSERT INTO sequent_backend.messaging_event_config
                 (tenant_id, election_event_id, config)
             VALUES ($1, $2, '{}')",
            &[&tenant, &event],
        )
        .await
        .unwrap();
    }
    tx.execute(
        "INSERT INTO sequent_backend.messaging_event_config
             (tenant_id, election_event_id, config)
         VALUES ($1, $2, '{}')",
        &[&configured.tenant, &configured.event],
    )
    .await
    .unwrap();
    // `configured.event`: counted long ago. `world.event`: pending.
    // `world.other_event`: never counted. `configured.other_event`: no
    // configuration.
    finish_recount(
        &tx,
        &configured.tenant,
        &configured.event,
        now - Duration::days(2),
        RecountOutcome::COUNTED,
        None,
    )
    .await
    .unwrap();
    let due = |at: chrono::DateTime<Utc>| {
        let tx = &tx;
        async move {
            events_due_for_recount(tx, at - Duration::hours(6), at - RECOUNT_PENDING_FOR, 100)
                .await
                .unwrap()
        }
    };
    assert_eq!(
        due(later).await,
        vec![
            (world.tenant, world.other_event),
            (configured.tenant, configured.event),
        ]
    );
    // A timed-out count waits for the next pass like any other; once its
    // pending recount is too old and its last count over six hours ago,
    // the event is due again.
    assert!(!due(later + minutes(31))
        .await
        .contains(&(world.tenant, world.event)));
    assert!(due(now + Duration::hours(7))
        .await
        .contains(&(world.tenant, world.event)));
    // A recent count isn't due.
    finish_recount(
        &tx,
        &world.tenant,
        &world.other_event,
        later,
        RecountOutcome::COUNTED,
        None,
    )
    .await
    .unwrap();
    assert!(!due(later)
        .await
        .contains(&(world.tenant, world.other_event)));
    assert_eq!(
        events_due_for_recount(&tx, later, later - RECOUNT_PENDING_FOR, 1)
            .await
            .unwrap()
            .len(),
        1
    );
    tx.rollback().await.unwrap();
}

/// A statement over its time limit is cancelled, and told apart from any
/// other failure, so a recount records "took longer" with its reason.
#[tokio::test]
async fn a_count_over_its_time_limit_is_a_timeout() {
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    set_statement_timeout(&tx, std::time::Duration::from_millis(1))
        .await
        .unwrap();
    let error = anyhow::Error::from(tx.execute("SELECT pg_sleep(1)", &[]).await.unwrap_err());
    assert!(is_statement_timeout(&error));
    assert!(is_statement_timeout(&error.context("counting voters")));
    tx.rollback().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let error = anyhow::Error::from(tx.execute("SELECT 1 / 0", &[]).await.unwrap_err());
    assert!(!is_statement_timeout(&error));
    tx.rollback().await.unwrap();
}

/// R8-05: the preview reads a configured account that was deleted as the
/// send does (`dispatch::account_for` refuses it), never as the
/// deployment's sender; an unconfigured email channel without a default
/// account is the deployment's.
#[tokio::test]
async fn a_deleted_configured_account_is_missing_in_the_preview() {
    use windmill::services::messaging::insights::{accounts_for_channels, ChannelAccount};
    let pool = schema::pool().await;
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    // A tenant with no account at all.
    let tenant = id(0x4a08, 1);
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
    let deleted = Uuid::new_v4().to_string();
    let config: EventMessagingConfig = serde_json::from_value(serde_json::json!({
        "version": 1,
        "channels": [{"channel": "SMS", "account_id": deleted, "purposes": ["NOTICE"]}],
    }))
    .unwrap();
    let accounts = accounts_for_channels(
        &tx,
        &tenant,
        Some(&config),
        &[MessageChannel::SMS, MessageChannel::EMAIL],
    )
    .await
    .unwrap();
    assert!(matches!(
        &accounts[&MessageChannel::SMS],
        ChannelAccount::Missing { account_id } if *account_id == deleted
    ));
    assert!(matches!(
        accounts[&MessageChannel::EMAIL],
        ChannelAccount::Unset
    ));
    tx.rollback().await.unwrap();
}
