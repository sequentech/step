// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use braid_messages::artifact::DkgPublicKey;
use braid_messages::artifact::{Ballots, Configuration};
use braid_messages::message::Message;
use braid_messages::newtypes::BatchNumber;
use braid_messages::newtypes::PublicKeyHash;
use braid_messages::newtypes::{TrusteeSet, MAX_TRUSTEES, NULL_TRUSTEE};
use braid_messages::protocol_manager::{ProtocolManager, ProtocolManagerConfig};
use braid_messages::statement::StatementType;

use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::StrandDeserialize;
use strand::serialization::StrandSerialize;
use strand::util::StrandError;

use anyhow::{Context, Result};
use std::env;
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
    board_name: &str,
) -> Result<C::E> {
    let mut board = BoardClient::new(&server_url, &user, &password).await?;

    let messages = board.get_messages(board_name, -1).await?;
    let pks_message = messages
        .into_iter()
        .map(|message| Message::strand_deserialize(&message.message))
        .find(|message| {
            if let Ok(m) = message {
                match m.statement.get_kind() {
                    StatementType::PublicKey => true,
                    _ => false,
                }
            } else {
                false
            }
        })
        .with_context(|| format!("Public Key not found on board {}", board_name))??;

    let bytes = pks_message.artifact.with_context(|| {
        format!(
            "Artifact missing on Public Key message on board {}",
            board_name
        )
    })?;
    let dkgpk = DkgPublicKey::<C>::strand_deserialize(&bytes).unwrap();
    Ok(dkgpk.pk)
}

pub fn get_configuration<C: Ctx>(messages: &Vec<Message>) -> Result<Configuration<C>> {
    let configuration_msg = messages
        .iter()
        .find(|message| {
            StatementType::Configuration == message.statement.get_kind()
                && message.artifact.is_some()
        })
        .unwrap();
    Ok(Configuration::<C>::strand_deserialize(
        &configuration_msg.artifact.clone().unwrap(),
    )?)
}

pub fn get_public_key_hash<C: Ctx>(messages: &Vec<Message>) -> Result<PublicKeyHash> {
    let public_key_message = messages
        .iter()
        .find(|message| {
            StatementType::PublicKey == message.statement.get_kind() && message.artifact.is_some()
        })
        .unwrap();
    let public_key_bytes = public_key_message.artifact.clone().unwrap();
    let dkgpk = DkgPublicKey::<C>::strand_deserialize(&public_key_bytes).unwrap();
    let pk_bytes = dkgpk.strand_serialize()?;
    let pk_h = strand::hash::hash_to_array(&pk_bytes)?;
    Ok(PublicKeyHash(strand::util::to_u8_array(&pk_h).unwrap()))
}

pub fn generate_full_trustee_set<C: Ctx>(configuration: &Configuration<C>) -> TrusteeSet {
    let mut selected_trustees: TrusteeSet = [NULL_TRUSTEE; MAX_TRUSTEES];
    for i in 0..configuration.trustees.len() {
        selected_trustees[i] = i + 1;
    }
    selected_trustees
}

pub async fn get_board_messages(board: &mut BoardClient, board_name: &str) -> Result<Vec<Message>> {
    let board_messages = board.get_messages(board_name, -1).await?;

    let messages: Vec<Message> = board_messages
        .iter()
        .map(|board_message| Message::strand_deserialize(&board_message.message))
        .collect::<Result<Vec<_>, StrandError>>()?;
    Ok(messages)
}

#[instrument(skip_all)]
pub async fn add_ballots_to_board<C: Ctx>(
    board_name: &str,
    ballots: Vec<Ciphertext<C>>,
    batch: BatchNumber,
) -> Result<()> {
    // 1. get env vars
    let user = env::var("IMMUDB_USER").expect(&format!("IMMUDB_USER must be set"));
    let password = env::var("IMMUDB_PASSWORD").expect(&format!("IMMUDB_PASSWORD must be set"));
    let server_url =
        env::var("IMMUDB_SERVER_URL").expect(&format!("IMMUDB_SERVER_URL must be set"));

    let pm = gen_protocol_manager::<C>();

    let mut board = BoardClient::new(&server_url, &user, &password).await?;
    let messages: Vec<Message> = get_board_messages(&mut board, board_name).await?;
    let configuration = get_configuration::<C>(&messages)?;
    let public_key_hash = get_public_key_hash::<C>(&messages)?;
    let selected_trustees: TrusteeSet = generate_full_trustee_set::<C>(&configuration);

    let message = Message::ballots_msg::<C, ProtocolManager<C>>(
        &configuration,
        batch,
        &Ballots::<C>::new(ballots),
        selected_trustees,
        public_key_hash,
        &pm,
    )?;
    info!("Adding configuration to the board..");
    let board_message: BoardMessage = message.try_into()?;
    board
        .insert_messages(board_name, &vec![board_message])
        .await
}
