// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use bulletin_board::service::{BulletinBoardServer, BulletinBoardService};
use bulletin_board::util::init_log;
use tonic::transport::Server;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_log()?;
    let bulletin_board_service = BulletinBoardService::read_config()?;
    let addr = bulletin_board_service.get_config().server_url.parse()?;

    info!(?addr, "Launching the bulletin board server");

    Server::builder()
        .add_service(BulletinBoardServer::new(bulletin_board_service))
        .serve(addr)
        .await?;

    Ok(())
}
