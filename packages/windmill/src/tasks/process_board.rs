// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use braid_messages::artifact::Plaintexts;
use braid_messages::message::Message;
use braid_messages::statement::Statement;
use celery::error::TaskError;
use sequent_core::services::keycloak;
use strand::backend::ristretto::RistrettoCtx;
use strand::serialization::StrandDeserialize;
use tracing::instrument;

use crate::hasura;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager;
use crate::types::error::Result;

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn process_board(election_event_id: String, tenant_id: String) -> Result<()> {
    // get credentials
    let auth_headers = keycloak::get_client_credentials().await?;

    // fetch election_event
    let hasura_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?;
    let election_event = &hasura_response
        .data
        .expect("expected data")
        .sequent_backend_election_event[0];

    // fetch tally_session_execution
    let hasura_response = hasura::tally_session_execution::get_tally_session_execution(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?;

    let res = &hasura_response.data.expect("expected data");
    dbg!(&res);

    let bulletin_board_opt =
        get_election_event_board(election_event.bulletin_board_reference.clone());

    if let Some(bulletin_board) = bulletin_board_opt {
        let pm = protocol_manager::gen_protocol_manager::<RistrettoCtx>();

        let mut board = protocol_manager::get_board().await?;
        
        // let messages: Vec<Message> =
        //     protocol_manager::get_board_messages(&mut board, &bulletin_board).await?;
        //
        // let res = messages
        //     .iter()
        //     .filter(|m| matches!(m.statement, Statement::Plaintexts(..)))
        //     .map(|m| {
        //         dbg!(&m);
        //         Message::strand_deserialize(&m.artifact.as_ref().unwrap().clone())
        //     })
        //     .collect::<Vec<_>>();

        let messages = board.get_messages(&bulletin_board, -1).await?;

        let messages = messages
            .into_iter()
            .map(|bm| Message::strand_deserialize(&bm.message))
            .filter(|bm| {
                if let Ok(m) = bm {
                    matches!(m.statement, Statement::Plaintexts(..))
                    // match m.statement.get_kind() {
                    //     StatementType::PublicKey => true,
                    //     _ => false,
                    // }
                } else {
                    false
                }
            })
            .map(|bm| {
                let mes = bm.unwrap();
                let artifact = mes.artifact.unwrap();
                let plaintexts = Plaintexts::<RistrettoCtx>::strand_deserialize(&artifact);
                plaintexts
            })
            .collect::<Vec<_>>();

        dbg!(&messages);
    }

    Ok(())
}
