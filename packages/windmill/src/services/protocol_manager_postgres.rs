// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use braid::protocol::board::grpc_m::GrpcB3;
use anyhow::{anyhow, Context, Result};
use tracing::{event, info, instrument, Level};
use std::env;

#[instrument(err)]
pub async fn get_board() -> Result<GrpcB3> {
    let server_url = env::var("BOARD_SERVER_URL").context("BOARD_SERVER_URL must be set")?;

    let board = GrpcB3::new(&server_url);

    Ok(board)
}
