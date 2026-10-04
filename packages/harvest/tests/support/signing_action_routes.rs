// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The routes of protected actions on the migrated test database: with the
//! action's rule on they answer `signing_request` and change nothing else;
//! with it off they run as before. Generating a publication cancels the
//! one waiting to be published.

use crate::route_services::rows::{self, Event};
use crate::route_services::{json, post, Services};
use crate::test_claims::Claims;
use chrono::Utc;
use deadpool_postgres::Pool;
use rocket::http::Status;
use sequent_core::signing::{
    RequesterSigning, SigningAction, SigningRequirement, SigningRule,
};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use uuid::Uuid;
use windmill::postgres::signing::upsert_signing_rule;

/// Two configurations: a Post label and how many sign.
const PRESETS: [(&str, u16); 2] = [("madrid-pe", 2), ("faculty-of-science", 3)];

fn ids(event: &Event) -> (Uuid, Uuid) {
    (
        Uuid::parse_str(&event.tenant_id).unwrap(),
        Uuid::parse_str(&event.election_event_id).unwrap(),
    )
}

async fn rule(
    pool: &Pool,
    event: &Event,
    action: SigningAction,
    signatures: u16,
) {
    let (tenant, election_event) = ids(event);
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    upsert_signing_rule(
        &tx,
        tenant,
        election_event,
        &SigningRule {
            action,
            requirement: SigningRequirement::Required,
            signatures,
            requester_signing: RequesterSigning::NotAllowed,
            expires_minutes: Some(60),
            revision: 0,
        },
        0,
        "manager",
        Some("Configuration Manager"),
    )
    .await
    .unwrap()
    .unwrap();
    tx.commit().await.unwrap();
}

/// An election of `event` with the Post label `label`.
async fn post_of(pool: &Pool, event: &Event, label: &str) -> String {
    let election = event.election(pool).await;
    rows::execute(
        pool,
        "UPDATE sequent_backend.election
         SET permission_label = $2, presentation = '{}' WHERE id = $1",
        &[&Uuid::parse_str(&election).unwrap(), &label],
    )
    .await;
    election
}

fn gold(event: &Event, roles: &[Permissions], label: &str) -> Claims {
    Claims::new(&event.tenant_id, "starter")
        .username("starter")
        .roles(roles.iter().cloned())
        .permission_labels(&[label])
        .acr(&Permissions::GOLD.to_string())
        .auth_time(Utc::now().timestamp())
}

async fn request(pool: &Pool, id: &Value) -> (String, String, Value, i32) {
    let id = Uuid::parse_str(id.as_str().unwrap()).unwrap();
    let row = rows::query(
        pool,
        "SELECT action, status, subject, required FROM sequent_backend.signing_request
         WHERE id = $1",
        &[&id],
    )
    .await
    .remove(0);
    (row.get(0), row.get(1), row.get(2), row.get(3))
}

/// Opens every channel of the Post.
async fn open_post(pool: &Pool, election: &str) {
    rows::execute(
        pool,
        "UPDATE sequent_backend.election SET status = $2 WHERE id = $1",
        &[
            &Uuid::parse_str(election).unwrap(),
            &json!({"voting_status": "OPEN"}),
        ],
    )
    .await;
}

/// A published event-level ballot publication of the Post; its id.
async fn published_for(pool: &Pool, event: &Event, election: &str) -> String {
    let (tenant, election_event) = ids(event);
    let id = Uuid::new_v4();
    rows::execute(
        pool,
        "INSERT INTO sequent_backend.ballot_publication
             (id, tenant_id, election_event_id, is_generated, election_ids, published_at)
         VALUES ($1, $2, $3, true, ARRAY[$4::uuid], now())",
        &[&id, &tenant, &election_event, &Uuid::parse_str(election).unwrap()],
    )
    .await;
    id.to_string()
}

async fn election_status(pool: &Pool, election: &str) -> Option<Value> {
    rows::query(
        pool,
        "SELECT status FROM sequent_backend.election WHERE id = $1",
        &[&Uuid::parse_str(election).unwrap()],
    )
    .await
    .remove(0)
    .get(0)
}

#[rocket::async_test]
async fn closing_a_post_answers_its_signing_request_and_changes_nothing() {
    for (label, required) in PRESETS {
        let services = Services::on_test_database().await;
        let client = services.client().await;
        let event = rows::event(&services.hasura).await;
        let election = post_of(&services.hasura, &event, label).await;
        rule(
            &services.hasura,
            &event,
            SigningAction::CloseVoting,
            required,
        )
        .await;
        open_post(&services.hasura, &election).await;
        let before = election_status(&services.hasura, &election).await;

        let (status, body) = json(
            post(
                &client,
                "/update-election-voting-status",
                &gold(&event, &[Permissions::ELECTION_STATE_WRITE], label),
                &json!({
                    "election_event_id": event.election_event_id,
                    "election_id": election,
                    "voting_status": "CLOSED",
                    "voting_channels": ["ONLINE"],
                }),
            )
            .await,
        )
        .await;
        assert_eq!(status, Status::Ok, "{body}");
        assert_eq!(body["election_id"], json!(election));
        let summary = &body["signing_request"];
        assert_eq!(summary["required"], json!(required));
        assert_eq!(summary["code"].as_str().map(str::len), Some(9));
        let (action, state, subject, needed) =
            request(&services.hasura, &summary["id"]).await;
        assert_eq!(action, "close-voting");
        assert_eq!(state, "waiting");
        assert_eq!(
            subject,
            json!({"channels": ["ONLINE"], "from": ["ONLINE=OPEN"]})
        );
        assert_eq!(needed, i32::from(required));
        assert_eq!(election_status(&services.hasura, &election).await, before);
    }
}

#[rocket::async_test]
async fn opening_a_post_with_the_rule_off_runs_the_status_change_as_before() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let election = post_of(&services.hasura, &event, "rule-off").await;
    let response = post(
        &client,
        "/update-election-voting-status",
        &gold(&event, &[Permissions::ELECTION_STATE_WRITE], "rule-off"),
        &json!({
            "election_event_id": event.election_event_id,
            "election_id": election,
            "voting_status": "OPEN",
            "voting_channels": ["ONLINE"],
        }),
    )
    .await;
    // The status service ran: this event has no bulletin board to post to.
    let status = response.status();
    let body = response.into_string().await.unwrap_or_default();
    assert_eq!(status, Status::InternalServerError);
    assert!(body.contains("bulletin board"), "{body}");
    assert_eq!(
        rows::query(
            &services.hasura,
            "SELECT count(*) FROM sequent_backend.signing_request WHERE election_event_id = $1",
            &[&ids(&event).1],
        )
        .await
        .remove(0)
        .get::<_, i64>(0),
        0
    );
}

#[rocket::async_test]
async fn initializing_a_post_answers_its_signing_request_without_a_tally() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let election = post_of(&services.hasura, &event, "init-post").await;
    rule(&services.hasura, &event, SigningAction::InitializeVoting, 2).await;
    let publication = published_for(&services.hasura, &event, &election).await;
    let (status, body) = json(
        post(
            &client,
            "/create-tally-ceremony",
            &gold(&event, &[Permissions::ADMIN_CEREMONY], "init-post"),
            &json!({
                "election_event_id": event.election_event_id,
                "election_ids": [election],
                "tally_type": "INITIALIZATION_REPORT",
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["tally_session_id"], Value::Null);
    let (action, _, subject, _) =
        request(&services.hasura, &body["signing_request"]["id"]).await;
    assert_eq!(action, "initialize-voting");
    assert_eq!(subject, json!({"publication_id": publication}));
    let sessions: i64 = rows::query(
        &services.hasura,
        "SELECT count(*) FROM sequent_backend.tally_session WHERE election_event_id = $1",
        &[&ids(&event).1],
    )
    .await
    .remove(0)
    .get(0);
    assert_eq!(sessions, 0);
}

#[rocket::async_test]
async fn publishing_waits_for_signatures_and_a_new_publication_cancels_the_request(
) {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    event.election(&services.hasura).await;
    rule(
        &services.hasura,
        &event,
        SigningAction::ApproveConfiguration,
        2,
    )
    .await;
    let publication = event
        .ballot_publication(&services.hasura, true, false)
        .await;
    let publisher = Claims::new(&event.tenant_id, "publisher")
        .roles([Permissions::PUBLISH_WRITE])
        .acr(&Permissions::GOLD.to_string())
        .auth_time(Utc::now().timestamp());

    let (status, body) = json(
        post(
            &client,
            "/publish-ballot",
            &publisher,
            &json!({
                "election_event_id": event.election_event_id,
                "ballot_publication_id": publication,
            }),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["ballot_publication_id"], json!(publication));
    let id = body["signing_request"]["id"].clone();
    let (action, state, subject, _) = request(&services.hasura, &id).await;
    assert_eq!(action, "approve-configuration");
    assert_eq!(state, "waiting");
    assert_eq!(subject["ballot_publication_id"], json!(publication));
    assert!(
        services.ledger.tasks().is_empty(),
        "no publish task while it waits"
    );
    let published: Option<chrono::DateTime<Utc>> = rows::query(
        &services.hasura,
        "SELECT published_at FROM sequent_backend.ballot_publication WHERE id = $1",
        &[&Uuid::parse_str(&publication).unwrap()],
    )
    .await
    .remove(0)
    .get(0);
    assert_eq!(published, None);

    let (status, body) = json(
        post(
            &client,
            "/generate-ballot-publication",
            &publisher,
            &json!({"election_event_id": event.election_event_id}),
        )
        .await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let (_, state, ..) = request(&services.hasura, &id).await;
    assert_eq!(state, "cancelled");
}

#[rocket::async_test]
async fn the_whole_event_does_not_open_or_close_while_a_post_needs_signatures()
{
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    rule(&services.hasura, &event, SigningAction::CloseVoting, 2).await;
    let claims = gold(&event, &[Permissions::ELECTION_STATE_WRITE], "any");
    let closing = json!({
        "election_event_id": event.election_event_id,
        "voting_status": "CLOSED",
    });
    let (status, body) = json(
        post(&client, "/update-event-voting-status", &claims, &closing).await,
    )
    .await;
    assert_eq!(status, Status::Conflict);
    assert_eq!(body["extensions"]["code"], json!("signing-required"));
    // Opening needs no signatures here, so the event-level change runs (and
    // fails only for want of a bulletin board).
    let opening = json!({
        "election_event_id": event.election_event_id,
        "voting_status": "OPEN",
    });
    let response =
        post(&client, "/update-event-voting-status", &claims, &opening).await;
    assert_ne!(response.status(), Status::Conflict);
}

/// An application of a Post's area, and its registry record in Keycloak.
async fn application_of(
    pool: &Pool,
    event: &Event,
    election: &str,
) -> (String, String) {
    let (tenant, election_event) = ids(event);
    let election = Uuid::parse_str(election).unwrap();
    let (area, contest, application) =
        (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    rows::execute(
        pool,
        "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
         VALUES ($1, $2, $3, 'Country')",
        &[&area, &tenant, &election_event],
    )
    .await;
    rows::execute(
        pool,
        "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
         VALUES ($1, $2, $3, $4)",
        &[&contest, &tenant, &election_event, &election],
    )
    .await;
    rows::execute(
        pool,
        "INSERT INTO sequent_backend.area_contest
             (id, tenant_id, election_event_id, area_id, contest_id)
         VALUES ($1, $2, $3, $4, $5)",
        &[&Uuid::new_v4(), &tenant, &election_event, &area, &contest],
    )
    .await;
    rows::execute(
        pool,
        "INSERT INTO sequent_backend.applications
             (id, tenant_id, election_event_id, area_id, applicant_id, status,
              verification_type, applicant_data, annotations, created_at)
         VALUES ($1, $2, $3, $4, 'applicant', 'PENDING', 'MANUAL', '{}', '{}',
                 '2028-03-02T10:00:00Z')",
        &[&application, &tenant, &election_event, &area],
    )
    .await;
    let realm = rows::keycloak_realm(
        pool,
        &format!("tenant-{tenant}-event-{election_event}"),
    )
    .await;
    let registry = rows::keycloak_user(pool, &realm, "ramon", true).await;
    (application.to_string(), registry)
}

#[rocket::async_test]
async fn approving_a_voter_answers_its_signing_request() {
    let services = Services::on_test_database().await;
    let client = services.client().await;
    let event = rows::event(&services.hasura).await;
    let election = post_of(&services.hasura, &event, "approvals").await;
    rule(&services.hasura, &event, SigningAction::ApproveVoter, 2).await;
    let (application, registry) =
        application_of(&services.hasura, &event, &election).await;
    // The OFOV staff member holds the Post's label: a labelled Post is
    // hidden from staff without it.
    let ofov = Claims::new(&event.tenant_id, "ofov")
        .username("ofov")
        .roles([Permissions::APPLICATION_WRITE])
        .permission_labels(&["approvals"]);
    let approval = json!({
        "tenant_id": event.tenant_id,
        "election_event_id": event.election_event_id,
        "id": application,
        "user_id": registry,
    });
    let (status, answer) = json(
        post(&client, "/change-application-status", &ofov, &approval).await,
    )
    .await;
    assert_eq!(status, Status::Ok, "{answer}");
    assert_eq!(answer["message"], Value::Null);
    let id = answer["signing_request"]["id"].clone();
    let (action, state, subject, _) = request(&services.hasura, &id).await;
    assert_eq!(action, "approve-voter");
    assert_eq!(state, "waiting");
    assert_eq!(subject["application_id"], json!(application));
    assert_eq!(subject["applicant_registry_id"], json!(registry));
    assert_eq!(subject["registry_record"], json!("ramon"));
    let application_status: String = rows::query(
        &services.hasura,
        "SELECT status FROM sequent_backend.applications WHERE id = $1",
        &[&Uuid::parse_str(&application).unwrap()],
    )
    .await
    .remove(0)
    .get(0);
    assert_eq!(application_status, "PENDING");
}

/// One short-send shape: a send refused for signatures answers 409 in the
/// contract's error shape, `signing-required` while signing applies and
/// `signatures-short` when a package without signing has too few uploaded
/// signatures.
#[test]
fn a_refused_send_answers_its_code() {
    use crate::routes::miru_plugin::transmission_failure;
    use windmill::services::consolidation::send_transmission_package_service::TransmissionSignaturesShort;
    use windmill::services::signing::actions::transmission::TransmissionRefusal;

    let short = transmission_failure(&TransmissionRefusal::SignaturesShort(
        TransmissionSignaturesShort {
            signatures: 1,
            threshold: 2,
        },
    ));
    assert_eq!(short.status, Status::Conflict);
    assert_eq!(short.code.to_string(), "signatures-short");
    assert!(
        short.message.contains("1 of the 2 signatures"),
        "{}",
        short.message
    );
    let unsigned = transmission_failure(&TransmissionRefusal::NotSigned {
        code: None,
        status: None,
    });
    assert_eq!(unsigned.status, Status::Conflict);
    assert_eq!(unsigned.code.to_string(), "signing-required");
}

/// Rows the key step's transaction wrote, on the connection that holds the
/// marker table.
async fn markers(client: &deadpool_postgres::Object) -> i64 {
    client
        .query_one("SELECT count(*) FROM key_step_marker", &[])
        .await
        .unwrap()
        .get(0)
}

/// A key step whose key share is not the trustee's records nothing and
/// answers `is_valid: false`; a step that ran, or that waits for the
/// trustee's signature, commits what it recorded and answers the request
/// to sign, if any.
#[rocket::async_test]
async fn a_key_step_commits_its_outcome_and_answers_the_request_to_sign() {
    use crate::routes::keys_ceremony::finish_key_share_step;
    use windmill::services::signing::guard::SigningRequestSummary;
    use windmill::services::signing::key_shares::KeyShareOutcome;

    let services = Services::on_test_database().await;
    let mut client = services.hasura.get().await.unwrap();
    client
        .batch_execute("CREATE TEMP TABLE key_step_marker (step text)")
        .await
        .unwrap();
    let summary = SigningRequestSummary {
        id: Uuid::new_v4(),
        code: "7F3A-91C2".to_string(),
        required: 1,
        expires_at: Some(Utc::now()),
    };
    for (step, outcome, answer, recorded) in [
        ("invalid", KeyShareOutcome::Invalid, (false, None), 0),
        ("done", KeyShareOutcome::Done(None), (true, None), 1),
        (
            "done with a request",
            KeyShareOutcome::Done(Some(json!({"request_id": summary.id}))),
            (true, None),
            2,
        ),
        (
            "waiting for a signature",
            KeyShareOutcome::SigningRequired(summary.clone()),
            (true, Some(summary.clone())),
            3,
        ),
    ] {
        let transaction = client.transaction().await.unwrap();
        transaction
            .execute("INSERT INTO key_step_marker VALUES ($1)", &[&step])
            .await
            .unwrap();
        let answered = finish_key_share_step(transaction, outcome).await;
        assert_eq!(answered, Ok(answer), "{step}");
        assert_eq!(markers(&client).await, recorded, "{step}");
    }
}
