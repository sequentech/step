// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use braid_messages::{artifact::Plaintexts, message::Message, statement::Statement};
use celery::prelude::TaskError;
use sequent_core::ballot::{Contest, ContestPresentation};
use sequent_core::ballot_codec::{BigUIntCodec, PlaintextCodec};
use sequent_core::services::keycloak;
use strand::{backend::ristretto::RistrettoCtx, serialization::StrandDeserialize};
use tracing::{event, instrument, Level};

use crate::hasura;
use crate::services::celery_app::get_celery_app;
use crate::services::election_event_board::get_election_event_board;
use crate::{services::protocol_manager, types::error::Result};

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn execute_tally_session(
    election_event_id: String,
    tenant_id: String,
    tally_session_id: String,
) -> Result<()> {
    // get credentials
    let auth_headers = keycloak::get_client_credentials().await?;

    // fetch election_event
    let election_events = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .expect("expected data")
    .sequent_backend_election_event;

    if 0 == election_events.len() {
        event!(
            Level::INFO,
            "Election Event not found {}",
            election_event_id.clone()
        );
        return Ok(());
    }

    let election_event = &election_events[0];

    let bulletin_board_opt =
        get_election_event_board(election_event.bulletin_board_reference.clone());

    if bulletin_board_opt.is_none() {
        event!(
            Level::INFO,
            "Election Event {} has no bulletin board",
            election_event_id.clone()
        );
        return Ok(());
    }

    let bulletin_board = bulletin_board_opt.unwrap();

    let tally_session_data = hasura::tally_session_execution::get_last_tally_session_execution(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        tally_session_id.clone(),
    )
    .await?
    .data
    .expect("expected data");

    // TODO: fetch contest from Hasura
    let contest = Contest {
        id: "63b1-f93b-4151-93d6-bbe0ea5eac46 69f2f987-460c-48ac-ac7a-4d44d99b37e6".into(),
        tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".into(),
        election_event_id: "33f18502-a67c-4853-8333-a58630663559".into(),
        election_id: "f2f1065e-b784-46d1-b81a-c71bfeb9ad55".into(),
        name: Some("Secretario General".into()),
        description: Some(
            "Elige quien quieres que sea tu Secretario General en tu municipio".into(),
        ),
        max_votes: 1,
        min_votes: 0,
        winning_candidates_num: 1,
        voting_type: Some("first-past-the-post".into()),
        counting_algorithm: Some("plurality-at-large".into()),
        is_encrypted: true,
        candidates: vec![],
        presentation: Some(ContestPresentation {
            allow_writeins: false,
            base32_writeins: true,
            invalid_vote_policy: "allowed".into(),
            cumulative_number_of_checkboxes: None,
            shuffle_categories: true,
            shuffle_all_options: true,
            shuffle_category_list: None,
            show_points: false,
            enable_checkable_lists: None,
        }),
    };

    let mut board_client = protocol_manager::get_board_client().await?;

    let messages = board_client.get_messages(&bulletin_board, -1).await?;

    let encoded_ballots = messages
        .into_iter()
        .map(|bm| Message::strand_deserialize(&bm.message))
        .filter_map(Result::ok)
        .filter(|m| matches!(m.statement, Statement::Plaintexts(..)))
        .map(|m| {
            // TODO: handle unwraps
            let artifact = m.artifact.unwrap();
            let res = Plaintexts::<RistrettoCtx>::strand_deserialize(&artifact).unwrap();
            let res = res.0 .0;
            let biguit_ballots = contest
                .decode_plaintext_contest_to_biguint(&res[0])
                .unwrap();

            // Testing decoded ballots here: to be removed
            let decoded_ballots = contest.decode_plaintext_contest_bigint(&biguit_ballots);
            dbg!(&decoded_ballots);

            biguit_ballots.to_str_radix(10)
        })
        .collect::<Vec<_>>();

    let path = PathBuf::from("/tmp/ballots.csv");
    let mut file = File::create(path).expect("Could not create file");
    let buffer = encoded_ballots.join("\n").into_bytes();
    file.write_all(&buffer).expect("Cannot written to file");

    Ok(())
}
