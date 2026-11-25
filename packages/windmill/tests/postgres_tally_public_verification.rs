// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The boards and manager messages the admin portal verifies a tally
//! session's bulletin boards with, over migrated, private rows.

#[path = "support/schema.rs"]
mod schema;

use base64::{engine::general_purpose::STANDARD, Engine};
use deadpool_postgres::Transaction;
use sequent_core::types::keycloak::VOTE_WEIGHT_BATCHES;
use serde_json::{json, Value};
use uuid::Uuid;
use windmill::services::tally_public_verification::{
    list_boards, TallyVerificationBoard, TallyVerificationBoards,
};

const CONFIGURATION: &[u8] = b"signed configuration";

struct World {
    tenant: Uuid,
    event: Uuid,
    other_event: Uuid,
    other_tenant: Uuid,
    session: Uuid,
    other_session: Uuid,
}

async fn execute(
    tx: &Transaction<'_>,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) {
    tx.execute(sql, params)
        .await
        .unwrap_or_else(|error| panic!("{sql}: {error:?}"));
}

async fn tally_session(tx: &Transaction<'_>, tenant: Uuid, event: Uuid, ceremony: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    execute(
        tx,
        "INSERT INTO sequent_backend.tally_session
         (id, tenant_id, election_event_id, keys_ceremony_id, threshold)
         VALUES ($1, $2, $3, $4, 1)",
        &[&id, &tenant, &event, &ceremony],
    )
    .await;
    id
}

/// The contest area of `session` whose batches start at `session_id`.
async fn contest_area(
    tx: &Transaction<'_>,
    w: &World,
    session: Uuid,
    session_id: i32,
    election: Uuid,
    contest: Option<Uuid>,
    area: Uuid,
) {
    execute(
        tx,
        "INSERT INTO sequent_backend.tally_session_contest
         (id, tenant_id, election_event_id, area_id, contest_id, session_id,
          tally_session_id, election_id)
         VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7)",
        &[
            &w.tenant,
            &w.event,
            &area,
            &contest,
            &session_id,
            &session,
            &election,
        ],
    )
    .await;
}

/// The tally board of `session` for `batch`, whose `Ballots` message is
/// `ballots`; returns its name.
async fn tally_board(
    tx: &Transaction<'_>,
    w: &World,
    dkg: Uuid,
    ceremony: Uuid,
    session: Uuid,
    batch: i32,
    ballots: &[u8],
) -> String {
    let name = format!("tally_{}_{batch}", session.simple());
    execute(
        tx,
        "INSERT INTO sequent_backend.protocol_board
         (tenant_id, election_event_id, parent_id, keys_ceremony_id,
          tally_session_id, batch, name, manager_message)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        &[
            &w.tenant, &w.event, &dkg, &ceremony, &session, &batch, &name, &ballots,
        ],
    )
    .await;
    name
}

/// A `presentation` holding an English name, as elections and contests keep
/// their names.
fn named(name: &str) -> Value {
    json!({"i18n": {"en": {"name": name}}})
}

fn expected(
    dkg: &str,
    tally: &str,
    batch: i32,
    names: [Option<&str>; 3],
    ballots: &[u8],
) -> TallyVerificationBoard {
    let [election, contest, area] = names;
    TallyVerificationBoard {
        dkg_board: dkg.to_owned(),
        tally_board: tally.to_owned(),
        batch,
        election_presentation: election.map(named),
        contest_presentation: contest.map(named),
        area_name: area.map(str::to_owned),
        configuration_message: STANDARD.encode(CONFIGURATION),
        ballots_message: STANDARD.encode(ballots),
    }
}

#[tokio::test]
async fn a_sessions_boards_come_by_batch_with_their_dkg_board_and_contest_area() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, other_tenant) = (Uuid::new_v4(), Uuid::new_v4());
    let (event, other_event) = (Uuid::new_v4(), Uuid::new_v4());
    for id in [tenant, other_tenant] {
        execute(
            &tx,
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&id, &id.to_string()],
        )
        .await;
    }
    for id in [event, other_event] {
        execute(
            &tx,
            "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
             VALUES ($1, $2, 'RSA256')",
            &[&id, &tenant],
        )
        .await;
    }
    let ceremony = Uuid::new_v4();
    execute(
        &tx,
        "INSERT INTO sequent_backend.keys_ceremony
         (id, tenant_id, election_event_id, trustee_ids, threshold)
         VALUES ($1, $2, $3, '{}', 1)",
        &[&ceremony, &tenant, &event],
    )
    .await;
    let dkg = Uuid::new_v4();
    let dkg_name = format!("dkg_{}", ceremony.simple());
    execute(
        &tx,
        "INSERT INTO sequent_backend.protocol_board
         (id, tenant_id, election_event_id, keys_ceremony_id, name, manager_message)
         VALUES ($1, $2, $3, $4, $5, $6)",
        &[&dkg, &tenant, &event, &ceremony, &dkg_name, &CONFIGURATION],
    )
    .await;
    let session = tally_session(&tx, tenant, event, ceremony).await;
    let other_session = tally_session(&tx, tenant, event, ceremony).await;
    let w = World {
        tenant,
        event,
        other_event,
        other_tenant,
        session,
        other_session,
    };

    let election = Uuid::new_v4();
    execute(
        &tx,
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, presentation)
         VALUES ($1, $2, $3, $4)",
        &[&election, &tenant, &event, &named("General election")],
    )
    .await;
    let contest = Uuid::new_v4();
    execute(
        &tx,
        "INSERT INTO sequent_backend.contest
         (id, tenant_id, election_event_id, election_id, presentation, max_votes, external_id)
         VALUES ($1, $2, $3, $4, $5, 1, $6)",
        &[
            &contest,
            &tenant,
            &event,
            &election,
            &named("Mayor"),
            &contest.to_string(),
        ],
    )
    .await;
    let mut areas = vec![];
    for name in ["North", "South"] {
        let area = Uuid::new_v4();
        execute(
            &tx,
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
             VALUES ($1, $2, $3, $4)",
            &[&area, &tenant, &event, &name],
        )
        .await;
        areas.push(area);
    }
    let stride = i32::try_from(VOTE_WEIGHT_BATCHES).unwrap();
    // North's contest, then South's ballots with every contest in one
    // ciphertext, which name no contest.
    contest_area(&tx, &w, session, 0, election, Some(contest), areas[0]).await;
    contest_area(&tx, &w, session, stride, election, None, areas[1]).await;
    // The same contest area in another session.
    contest_area(&tx, &w, other_session, 0, election, Some(contest), areas[0]).await;

    // Inserted out of order: a weight batch of North, South, North's first
    // batch, and a batch no contest area of the session owns.
    let north_weighted = tally_board(&tx, &w, dkg, ceremony, session, 1, b"north 2").await;
    let south = tally_board(&tx, &w, dkg, ceremony, session, stride, b"south").await;
    let north = tally_board(&tx, &w, dkg, ceremony, session, 0, b"north 1").await;
    let unowned = tally_board(&tx, &w, dkg, ceremony, session, 3 * stride, b"unowned").await;
    tally_board(&tx, &w, dkg, ceremony, other_session, 0, b"another session").await;

    let read = |tenant: Uuid, event: Uuid, session: Uuid| {
        let tx = &tx;
        async move {
            list_boards(
                tx,
                &tenant.to_string(),
                &event.to_string(),
                &session.to_string(),
            )
            .await
            .expect("the boards")
        }
    };
    let north_names = [Some("General election"), Some("Mayor"), Some("North")];
    assert_eq!(
        read(w.tenant, w.event, w.session).await,
        TallyVerificationBoards {
            boards: vec![
                expected(&dkg_name, &north, 0, north_names, b"north 1"),
                expected(&dkg_name, &north_weighted, 1, north_names, b"north 2"),
                expected(
                    &dkg_name,
                    &south,
                    stride,
                    [Some("General election"), None, Some("South")],
                    b"south"
                ),
                expected(&dkg_name, &unowned, 3 * stride, [None; 3], b"unowned"),
            ],
        }
    );
    assert_eq!(
        read(w.tenant, w.event, w.other_session).await.boards.len(),
        1
    );
    // The session is matched within its own tenant and event only.
    assert!(read(w.tenant, w.other_event, w.session)
        .await
        .boards
        .is_empty());
    assert!(read(w.other_tenant, w.event, w.session)
        .await
        .boards
        .is_empty());
}

#[tokio::test]
async fn ids_that_are_not_version_4_uuids_and_database_failures_are_errors() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let id = Uuid::new_v4().to_string();
    for (tenant, event, session) in [
        ("not-a-uuid", id.as_str(), id.as_str()),
        (id.as_str(), "not-a-uuid", id.as_str()),
        (id.as_str(), id.as_str(), "not-a-uuid"),
    ] {
        assert!(list_boards(&tx, tenant, event, session).await.is_err());
    }
    // A failed statement aborts the transaction, so the query fails.
    assert!(tx.execute("SELECT 1 / 0", &[]).await.is_err());
    assert!(list_boards(&tx, &id, &id, &id).await.is_err());
}
