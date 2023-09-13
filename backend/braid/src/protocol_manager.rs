// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::protocol2::artifact::Configuration;
use crate::protocol2::message::Message;
use crate::protocol2::trustee::ProtocolManager;
use crate::run::config::ProtocolManagerConfig;

use strand::context::Ctx;
use strand::rnd::StrandRng;

use anyhow::Result;
use std::marker::PhantomData;
use tracing::{info, instrument};

use immu_board::{BoardClient, BoardMessage};
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

pub fn gen_protocol_manager<C: Ctx>() -> ProtocolManager<C> {
    let mut csprng = StrandRng;

    let pmkey: StrandSignatureSk = StrandSignatureSk::new(&mut csprng);
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

pub async fn add_config_to_board<C: Ctx>(
    server_url: &str,
    user: &str,
    password: &str,
    threshold: usize,
    board_name: &str,
    trustee_pks: Vec<StrandSignaturePk>,
    pm: ProtocolManager<C>
) -> Result<()> {
    let configuration = Configuration::<C>::new(
        0,
        StrandSignaturePk::from(&pm.signing_key),
        trustee_pks,
        threshold,
        PhantomData,
    );

    let mut board = BoardClient::new(&server_url, &user, &password).await?;

    init(&mut board, configuration, pm, board_name).await
}

