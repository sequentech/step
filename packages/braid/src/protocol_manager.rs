// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::protocol2::artifact::Configuration;
use crate::protocol2::message::Message;
use crate::protocol2::trustee::ProtocolManager;
use crate::run::config::ProtocolManagerConfig;
use crate::protocol2::board::immudb::ImmudbBoard;
use crate::util::assert_folder;

use strand::context::Ctx;

use anyhow::Result;
use std::marker::PhantomData;
use tracing::{info, instrument};

use immu_board::{BoardClient, BoardMessage};
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

pub fn gen_protocol_manager<C: Ctx>() -> ProtocolManager<C> {
    let pmkey: StrandSignatureSk = StrandSignatureSk::gen().unwrap();
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };
    pm
}

pub fn serialize_protocol_manager<C: Ctx>(pm: &ProtocolManager<C>) -> String {
    let pmc = ProtocolManagerConfig::from(&pm);
    toml::to_string(&pmc).unwrap()
}

#[instrument]
async fn init<C: Ctx>(
    board: &mut BoardClient,
    configuration: Configuration<C>,
    pm: ProtocolManager<C>,
    board_name: &str,
) -> Result<()> {
    let message: BoardMessage = Message::bootstrap_msg(&configuration, &pm)?.try_into()?;
    info!("Adding configuration to the board..");
    board.insert_messages(board_name, &vec![message]).await
}

#[instrument(skip(user, password, pm))]
pub async fn add_config_to_board<C: Ctx>(
    server_url: &str,
    user: &str,
    password: &str,
    threshold: usize,
    board_name: &str,
    trustee_pks: Vec<StrandSignaturePk>,
    pm: ProtocolManager<C>,
) -> Result<()> {
    let configuration = Configuration::<C>::new(
        0,
        StrandSignaturePk::from(&pm.signing_key)?,
        trustee_pks,
        threshold,
        PhantomData,
    );

    let mut board = BoardClient::new(&server_url, &user, &password).await?;

    init(&mut board, configuration, pm, board_name).await
}


#[instrument(skip(user, password))]
pub async fn get_board_public_key<C: Ctx>(
    server_url: &str,
    user: &str,
    password: &str,
    board_name: &str
) -> Result<()> {
    let store_root = std::env::current_dir().unwrap().join("message_store");
    assert_folder(store_root.clone())?;
    let mut board = ImmudbBoard::new(
        server_url,
        user,
        password,
        board_name.to_string(),
        store_root.clone(),
    )
    .await?;
    let messages = board.get_messages(-1).await?;
    Ok(())
}