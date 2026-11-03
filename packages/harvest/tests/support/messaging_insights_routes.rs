// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The messaging insight routes: each refuses a caller without its
//! permission before any backend is reached, refuses malformed input with a
//! reason, and is reachable from Hasura only for the roles that hold that
//! permission. The counts themselves are tested in windmill
//! (`services::messaging::insights` and `postgres_messaging_insights`).

use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, Services};
use crate::test_claims::Claims;
use rocket::http::Status;
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;

const TENANT_ID: &str = "00000000-0000-4000-8000-00000000000a";
const EVENT_ID: &str = "00000000-0000-4000-8000-00000000000e";
const USER_ID: &str = "3f2b8c1e-6d4a-4c1b-9e7f-2a5d8b0c9e1a";

fn caller(permissions: &[Permissions]) -> Claims {
    Claims::new(TENANT_ID, USER_ID).roles(permissions.iter().cloned())
}

/// Each route, a valid body, and the permissions any one of which opens it.
fn routes() -> Vec<(&'static str, Value, Vec<Permissions>)> {
    vec![
        (
            "/messaging/event-stats",
            json!({"election_event_id": EVENT_ID}),
            vec![Permissions::MESSAGING_CONFIG_READ],
        ),
        (
            "/messaging/audience-preview",
            json!({"election_event_id": EVENT_ID, "audience_selection": "ALL_USERS"}),
            vec![Permissions::MESSAGING_CONFIG_READ],
        ),
        (
            "/messaging/voter-counts",
            json!({"election_event_id": EVENT_ID}),
            vec![Permissions::MESSAGING_CONFIG_READ],
        ),
        (
            "/messaging/recount-voters",
            json!({"election_event_id": EVENT_ID}),
            vec![Permissions::MESSAGING_CONFIG_READ],
        ),
        (
            "/messaging/monthly-volumes",
            json!({"month": "2026-10"}),
            vec![
                Permissions::MESSAGING_ACCOUNT_READ,
                Permissions::MESSAGING_CONFIG_READ,
            ],
        ),
        (
            "/applications/support-reference",
            json!({"election_event_id": EVENT_ID, "reference": "65SR-MSSD"}),
            vec![Permissions::APPLICATION_READ],
        ),
    ]
}

#[rocket::async_test]
async fn insights_need_their_permission_before_reaching_a_backend() {
    let client = Services::without_database().client().await;
    for (path, body, permissions) in routes() {
        // Write permissions and the other reads don't open a read.
        let (status, refusal) = json(
            post(
                &client,
                path,
                &caller(&[
                    Permissions::MESSAGING_CONFIG_WRITE,
                    Permissions::MESSAGING_ACCOUNT_WRITE,
                    Permissions::VOTER_READ,
                ]),
                &body,
            )
            .await,
        )
        .await;
        assert_eq!(status, Status::Unauthorized, "{path}: {refusal}");
        assert!(refusal["message"].is_string(), "{path}: {refusal}");
        // Any one of its permissions gets past authorization, to the
        // database the test leaves unreachable.
        for permission in permissions {
            let (status, answer) = json(
                post(&client, path, &caller(&[permission.clone()]), &body)
                    .await,
            )
            .await;
            assert_eq!(
                status,
                Status::InternalServerError,
                "{path} with {permission}: {answer}"
            );
            assert_eq!(answer, json!({"message": "internal error"}), "{path}");
        }
    }
}

#[rocket::async_test]
async fn malformed_input_is_refused_with_its_reason() {
    let client = Services::without_database().client().await;
    let cases = [
        (
            "/messaging/monthly-volumes",
            json!({"month": "10/2026"}),
            Permissions::MESSAGING_ACCOUNT_READ,
            "The month must be written as YYYY-MM",
        ),
        (
            "/applications/support-reference",
            json!({"election_event_id": EVENT_ID, "reference": "dela cruz"}),
            Permissions::APPLICATION_READ,
            "Not a support reference",
        ),
        (
            "/messaging/audience-preview",
            json!({"election_event_id": EVENT_ID, "audience_selection": "SELECTED", "voter_ids": []}),
            Permissions::MESSAGING_CONFIG_READ,
            "Select at least one voter",
        ),
        (
            "/messaging/audience-preview",
            json!({"election_event_id": EVENT_ID, "audience_selection": "EVERYONE"}),
            Permissions::MESSAGING_CONFIG_READ,
            "Unknown audience",
        ),
        (
            "/messaging/event-stats",
            json!({"election_event_id": "not-an-id"}),
            Permissions::MESSAGING_CONFIG_READ,
            "invalid identifier",
        ),
    ];
    for (path, body, permission, reason) in cases {
        let (status, answer) =
            json(post(&client, path, &caller(&[permission]), &body).await)
                .await;
        assert_eq!(status, Status::BadRequest, "{path}: {answer}");
        assert_eq!(answer, json!({"message": reason}), "{path}");
    }
}

fn metadata(file: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../hasura/metadata")
            .join(file),
    )
    .unwrap()
}

/// Hasura forwards each action to its route, only for the roles named.
#[test]
fn hasura_actions_reach_the_routes_for_their_permissions_only() {
    let actions: serde_yaml::Value =
        serde_yaml::from_str(&metadata("actions.yaml")).unwrap();
    let graphql = metadata("actions.graphql");
    for (action, handler, kind, roles) in [
        (
            "messaging_event_stats",
            "/messaging/event-stats",
            "query",
            vec!["messaging-config-read"],
        ),
        (
            "messaging_voter_counts",
            "/messaging/voter-counts",
            "query",
            vec!["messaging-config-read"],
        ),
        (
            "messaging_recount_voters",
            "/messaging/recount-voters",
            "mutation",
            vec!["messaging-config-read"],
        ),
        (
            "messaging_audience_preview",
            "/messaging/audience-preview",
            "query",
            vec!["messaging-config-read"],
        ),
        (
            "messaging_monthly_volumes",
            "/messaging/monthly-volumes",
            "query",
            vec!["messaging-account-read", "messaging-config-read"],
        ),
        (
            "find_applications_by_support_reference",
            "/applications/support-reference",
            "query",
            vec!["application-read"],
        ),
    ] {
        let entry = actions["actions"]
            .as_sequence()
            .unwrap()
            .iter()
            .find(|entry| entry["name"].as_str() == Some(action))
            .unwrap_or_else(|| panic!("{action} is not in actions.yaml"));
        assert_eq!(
            entry["definition"]["handler"].as_str(),
            Some(format!("http://{{{{HARVEST_DOMAIN}}}}{handler}").as_str()),
            "{action}"
        );
        // Hasura names a query's type; a mutation has none.
        assert_eq!(
            entry["definition"]["type"].as_str(),
            (kind == "query").then_some("query"),
            "{action}"
        );
        let granted: BTreeSet<&str> = entry["permissions"]
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(|permission| permission["role"].as_str())
            .collect();
        assert_eq!(granted, roles.into_iter().collect(), "{action}");
        assert!(
            graphql.contains(&format!("    {action}(")),
            "{action} is not declared in actions.graphql"
        );
    }
}

// --- Successful answers on the test database: the JSON the admin queries
// read (field names, counts), and nothing a destination could be read from.

/// What the ledger rows carry that no answer may show.
const MASKED_DESTINATION: &str = "+63•••4567";
const DESTINATION_DIGEST: &str = "digest-of-the-destination";

fn reader(event: &Event, permission: Permissions) -> Claims {
    Claims::new(&event.tenant_id, USER_ID).roles([permission])
}

fn ids(event: &Event) -> (uuid::Uuid, uuid::Uuid) {
    (
        uuid::Uuid::parse_str(&event.tenant_id).unwrap(),
        uuid::Uuid::parse_str(&event.election_event_id).unwrap(),
    )
}

/// One attempt of a notice, as the ledger writes it.
#[allow(clippy::too_many_arguments)]
async fn message(
    services: &Services,
    event: &Event,
    key: &str,
    channel: Option<&str>,
    account: Option<uuid::Uuid>,
    state: &str,
    at: chrono::DateTime<chrono::Utc>,
    dispatched: bool,
) {
    let (tenant, election_event) = ids(event);
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.message
            (tenant_id, election_event_id, account_id, channel, direction, purpose,
             masked_destination, destination_digest, destination_country, logical_key,
             attempt, state, billing_category, billable, created_at, dispatched_at)
         VALUES ($1, $2, $3, $4, 'OUTBOUND', 'NOTICE', $5, $6, 'PH', $7, 1, $8,
                 'UTILITY', true, $9, $10)",
        &[
            &tenant,
            &election_event,
            &account,
            &channel,
            &MASKED_DESTINATION,
            &DESTINATION_DIGEST,
            &key,
            &state,
            &at,
            &dispatched.then_some(at),
        ],
    )
    .await;
}

/// The event's voters as last counted: Email (all offered), WhatsApp (no
/// voter's Post offers it) and voters with no channel.
async fn counted(
    services: &Services,
    event: &Event,
) -> chrono::DateTime<chrono::Utc> {
    use sequent_core::types::messaging::MessageChannel;
    use windmill::postgres::messaging_insights::{
        finish_recount, replace_voter_counts, CachedVoterCount, RecountOutcome,
        VoterCountScope,
    };
    let (tenant, election_event) = ids(event);
    let at = chrono::DateTime::parse_from_rfc3339("2026-10-09T08:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let row = |scope, channel, voters, not_offered| CachedVoterCount {
        scope,
        area_id: None,
        channel,
        voters,
        not_offered,
    };
    let mut client = services.hasura.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    replace_voter_counts(
        &tx,
        &tenant,
        &election_event,
        &[
            row(VoterCountScope::ALL, Some(MessageChannel::EMAIL), 1200, 0),
            row(VoterCountScope::ALL, Some(MessageChannel::WHATSAPP), 2, 2),
            row(VoterCountScope::ALL, None, 1, 0),
            row(VoterCountScope::VOTED, Some(MessageChannel::EMAIL), 5, 0),
        ],
        at,
    )
    .await
    .unwrap();
    finish_recount(
        &tx,
        &tenant,
        &election_event,
        at,
        RecountOutcome::COUNTED,
        None,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    at
}

#[rocket::async_test]
async fn voter_counts_answer_the_last_count_with_when() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let body = json!({"election_event_id": event.election_event_id});
    let caller = reader(&event, Permissions::MESSAGING_CONFIG_READ);
    // Never counted: no counts, and no time.
    let (status, answer) =
        json(post(&client, "/messaging/voter-counts", &caller, &body).await)
            .await;
    assert_eq!(status, Status::Ok, "{answer}");
    assert_eq!(
        answer,
        json!({"total": 0, "channels": [],
               "status": {"computed_at": null, "pending": false, "outcome": null}})
    );
    counted(&services, &event).await;
    let (status, answer) =
        json(post(&client, "/messaging/voter-counts", &caller, &body).await)
            .await;
    assert_eq!(status, Status::Ok, "{answer}");
    assert_eq!(
        answer,
        json!({
            "total": 1203,
            // The event tab places each row in channel order itself.
            "channels": [
                {"channel": "EMAIL", "voters": 1200, "not_offered": 0},
                {"channel": "WHATSAPP", "voters": 2, "not_offered": 2},
                {"channel": null, "voters": 1, "not_offered": 0},
            ],
            "status": {"computed_at": "2026-10-09T08:00:00+00:00",
                       "pending": false, "outcome": "COUNTED"},
        })
    );
}

#[rocket::async_test]
async fn the_audience_preview_reads_the_last_count_with_who_sends() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    counted(&services, &event).await;
    let caller = reader(&event, Permissions::MESSAGING_CONFIG_READ);
    let (status, answer) = json(
        post(
            &client,
            "/messaging/audience-preview",
            &caller,
            &json!({"election_event_id": event.election_event_id,
                    "audience_selection": "ALL_USERS"}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{answer}");
    // No configuration: Email and SMS are offered, through the deployment's
    // sender (the tenant has no account); WhatsApp is shown only because
    // voters are on it, and nothing is sent there.
    assert_eq!(
        answer,
        json!({
            "total": 1203,
            "will_send": 1200,
            "channels": [
                {"channel": "WHATSAPP", "voters": 2, "not_offered": 2,
                 "sends_from": null, "account_id": null, "account_name": null},
                {"channel": "SMS", "voters": 0, "not_offered": 0,
                 "sends_from": "DEPLOYMENT", "account_id": null, "account_name": null},
                {"channel": "EMAIL", "voters": 1200, "not_offered": 0,
                 "sends_from": "DEPLOYMENT", "account_id": null, "account_name": null},
                {"channel": null, "voters": 1, "not_offered": 0,
                 "sends_from": null, "account_id": null, "account_name": null},
            ],
            "status": {"computed_at": "2026-10-09T08:00:00+00:00",
                       "pending": false, "outcome": "COUNTED"},
        })
    );
    // The voted filter reads its own counts.
    let (_, voted) = json(
        post(
            &client,
            "/messaging/audience-preview",
            &caller,
            &json!({"election_event_id": event.election_event_id,
                    "audience_selection": "VOTED"}),
        )
        .await,
    )
    .await;
    assert_eq!(voted["total"], 5);
    assert_eq!(voted["will_send"], 5);
}

#[rocket::async_test]
async fn a_recount_is_asked_for_once_while_it_is_on_its_way() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let body = json!({"election_event_id": event.election_event_id});
    let caller = reader(&event, Permissions::MESSAGING_CONFIG_READ);
    let (status, answer) =
        json(post(&client, "/messaging/recount-voters", &caller, &body).await)
            .await;
    assert_eq!(status, Status::Ok, "{answer}");
    assert_eq!(
        answer,
        json!({"requested": true,
               "status": {"computed_at": null, "pending": true, "outcome": null}})
    );
    let (_, again) =
        json(post(&client, "/messaging/recount-voters", &caller, &body).await)
            .await;
    assert_eq!(again["requested"], false);
    assert_eq!(again["status"]["pending"], true);
    let sent = services.tasks.sent();
    assert_eq!(sent.len(), 1, "one recount task for two asks");
    assert_eq!(sent[0].name, "recount_messaging_voters");
    assert_eq!(
        sent[0].kwargs,
        json!({"tenant_id": event.tenant_id,
               "election_event_id": event.election_event_id})
    );
    // The voter counts say it's on its way.
    let (_, counts) =
        json(post(&client, "/messaging/voter-counts", &caller, &body).await)
            .await;
    assert_eq!(counts["status"]["pending"], true);
}

#[rocket::async_test]
async fn event_stats_count_messages_and_never_show_a_destination() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let now = chrono::Utc::now();
    let hour = chrono::Duration::hours(1);
    message(
        &services,
        &event,
        "k:1",
        Some("SMS"),
        None,
        "DELIVERED",
        now - hour,
        true,
    )
    .await;
    message(
        &services,
        &event,
        "k:2",
        Some("SMS"),
        None,
        "FAILED",
        now - hour,
        true,
    )
    .await;
    message(
        &services,
        &event,
        "k:3",
        None,
        None,
        "FAILED",
        now - hour,
        false,
    )
    .await;
    let response = post(
        &client,
        "/messaging/event-stats",
        &reader(&event, Permissions::MESSAGING_CONFIG_READ),
        &json!({"election_event_id": event.election_event_id}),
    )
    .await;
    let status = response.status();
    let text = response.into_string().await.unwrap_or_default();
    assert_eq!(status, Status::Ok, "{text}");
    assert!(
        !text.contains(MASKED_DESTINATION)
            && !text.contains(DESTINATION_DIGEST)
    );
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        answer,
        json!({"channels": [
            {"channel": "SMS", "sent_last_24h": 2, "queued": 0, "accepted": 0,
             "delivered": 1, "failed": 1, "unknown": 0, "unresolved": 0},
            {"channel": null, "sent_last_24h": 1, "queued": 0, "accepted": 0,
             "delivered": 0, "failed": 1, "unknown": 0, "unresolved": 0},
        ]})
    );
}

#[rocket::async_test]
async fn monthly_volumes_answer_attempts_by_account_and_billing() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let (tenant, _) = ids(&event);
    let account = uuid::Uuid::new_v4();
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.messaging_account
            (id, tenant_id, channel, provider, name, webhook_key)
         VALUES ($1, $2, 'WHATSAPP', 'WHATSAPP_CLOUD_API', 'Election office', $3)",
        &[&account, &tenant, &account.simple().to_string()],
    )
    .await;
    let october = chrono::DateTime::parse_from_rfc3339("2026-10-05T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    for key in ["k:1", "k:2"] {
        message(
            &services,
            &event,
            key,
            Some("WHATSAPP"),
            Some(account),
            "DELIVERED",
            october,
            true,
        )
        .await;
    }
    // Never handed to a provider: not billed, not counted.
    message(
        &services,
        &event,
        "k:3",
        Some("WHATSAPP"),
        Some(account),
        "FAILED",
        october,
        false,
    )
    .await;
    let response = post(
        &client,
        "/messaging/monthly-volumes",
        &reader(&event, Permissions::MESSAGING_ACCOUNT_READ),
        &json!({"month": "2026-10"}),
    )
    .await;
    let status = response.status();
    let text = response.into_string().await.unwrap_or_default();
    assert_eq!(status, Status::Ok, "{text}");
    assert!(
        !text.contains(MASKED_DESTINATION)
            && !text.contains(DESTINATION_DIGEST)
    );
    let answer: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        answer,
        json!({"month": "2026-10", "rows": [
            {"channel": "WHATSAPP", "account_id": account.to_string(),
             "account_name": "Election office", "destination_country": "PH",
             "billing_category": "UTILITY", "billable": true, "messages": 2},
        ]})
    );
}

#[rocket::async_test]
async fn the_support_reference_answers_the_application_id_only() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let (tenant, election_event) = ids(&event);
    // The vector Keycloak's SupportReferenceTest uses: 65SR-MSSD.
    let application =
        uuid::Uuid::parse_str("7f0c2a9e-1b3d-4c5e-8f60-123456789abc").unwrap();
    rows::execute(
        &services.hasura,
        "INSERT INTO sequent_backend.applications
            (id, tenant_id, election_event_id, area_id, applicant_id, status,
             verification_type, applicant_data, permission_label)
         VALUES ($1, $2, $3, $4, 'voter', 'PENDING', 'MANUAL',
                 '{\"email\": \"voter@example.org\"}', 'post-a')",
        &[
            &application,
            &tenant,
            &election_event,
            &uuid::Uuid::new_v4(),
        ],
    )
    .await;
    let (status, answer) = json(
        post(
            &client,
            "/applications/support-reference",
            &reader(&event, Permissions::APPLICATION_READ),
            &json!({"election_event_id": event.election_event_id,
                    "reference": "65sr mssd"}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{answer}");
    assert_eq!(
        answer,
        json!({"reference": "65SR-MSSD", "application_ids": [application.to_string()]})
    );
}
