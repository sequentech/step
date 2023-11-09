// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{services::protocol_manager, types::error::Result};
use braid_messages::{artifact::Plaintexts, message::Message, statement::Statement};
use celery::prelude::TaskError;
use immu_board::BoardMessage;
use strand::{backend::ristretto::RistrettoCtx, serialization::StrandDeserialize};
use tracing::instrument;

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn process_ballots_from_messages(bulletin_board: String) -> Result<()> {
    let mut board = protocol_manager::get_board().await?;

    let messages = board.get_messages(&bulletin_board, -1).await?;

    let messages = messages
        .into_iter()
        .map(|bm| Message::strand_deserialize(&bm.message))
        .filter(|bm| {
            if let Ok(m) = bm {
                matches!(m.statement, Statement::Plaintexts(..))
            } else {
                false
            }
        })
        .map(|bm| {
            let mes = bm.unwrap();
            let artifact = mes.artifact.unwrap();

            Plaintexts::<RistrettoCtx>::strand_deserialize(&artifact)
        })
        .collect::<Vec<_>>();

    dbg!(&messages);
    
    Ok(())
}
