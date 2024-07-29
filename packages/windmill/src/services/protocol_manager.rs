// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use board_messages::braid::artifact::{Ballots, Channel, Configuration, DkgPublicKey};
use board_messages::braid::message::Message;
use board_messages::braid::newtypes::BatchNumber;
use board_messages::braid::newtypes::PublicKeyHash;
use board_messages::braid::newtypes::{TrusteeSet, MAX_TRUSTEES, NULL_TRUSTEE};
use board_messages::braid::protocol_manager::{ProtocolManager, ProtocolManagerConfig};
use board_messages::braid::statement::StatementType;

use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::StrandDeserialize;
use strand::serialization::StrandSerialize;
use strand::symm::EncryptionData;
use strand::util::StrandError;

use anyhow::{anyhow, Context, Result};
use std::env;
use std::marker::PhantomData;
use tracing::{event, info, instrument, Level};

use crate::services::vault;
use immu_board::{BoardClient, BoardMessage};
use immudb_rs::{sql_value::Value, Client, NamedParam, SqlValue};
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

pub fn get_protocol_manager_secret_path(board_name: &str) -> String {
    format!("boards/{board_name}/protocol-manager")
}

#[instrument(err)]
pub async fn create_protocol_manager_keys(board_name: &str) -> Result<()> {
    // create protocol manager keys
    let protocol_manager = gen_protocol_manager::<RistrettoCtx>();

    // save protocol manager keys in vault
    let protocol_config = serialize_protocol_manager::<RistrettoCtx>(&protocol_manager);
    vault::save_secret(
        get_protocol_manager_secret_path(board_name),
        protocol_config,
    )
    .await?;
    Ok(())
}

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

pub fn deserialize_protocol_manager<C: Ctx>(contents: String) -> ProtocolManager<C> {
    let pmc: ProtocolManagerConfig = toml::from_str(&contents).unwrap();
    let pmkey = pmc.get_signing_key().unwrap();
    ProtocolManager::new(pmkey)
}

#[instrument(err)]
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

#[instrument(skip(pm), err)]
pub async fn add_config_to_board<C: Ctx>(
    threshold: usize,
    board_name: &str,
    trustee_pks: Vec<StrandSignaturePk>,
    pm: ProtocolManager<C>,
) -> Result<()> {
    let configuration = Configuration::<C>::new(
        0,
        StrandSignaturePk::from_sk(&pm.signing_key)?,
        trustee_pks,
        threshold,
        PhantomData,
    );

    let mut board_client = get_board_client().await?;

    init(&mut board_client, configuration, pm, board_name).await
}

#[instrument(err)]
pub async fn get_board_public_key<C: Ctx>(board_name: &str) -> Result<C::E> {
    let mut board = get_board_client().await?;

    let board_messages = board.get_messages(board_name, -1).await?;

    let valid_statements = vec![StatementType::PublicKey, StatementType::PublicKeySigned];
    let messages: Vec<Message> = board_messages
        .into_iter()
        .filter_map(|board_message| Message::strand_deserialize(&board_message.message).ok())
        .collect();

    let config = get_configuration::<C>(&messages)?;

    config
        .trustees
        .into_iter()
        .map(|trustee_signature| {
            let trustee_pk = messages.iter().any(|message| {
                message.sender.pk == trustee_signature
                    && valid_statements.contains(&message.statement.get_kind())
            });
            if trustee_pk {
                Ok(())
            } else {
                Err(anyhow!(
                    "Missing public key for trustee {:?}",
                    trustee_signature
                ))
            }
        })
        .collect::<Result<()>>()?;

    let pks_message = messages
        .into_iter()
        .find(|message| StatementType::PublicKey == message.statement.get_kind())
        .with_context(|| format!("Public Key not found on board {}", board_name))?;

    let bytes = pks_message.artifact.with_context(|| {
        format!(
            "Artifact missing on Public Key message on board {}",
            board_name
        )
    })?;
    let dkgpk = DkgPublicKey::<C>::strand_deserialize(&bytes).unwrap();
    Ok(dkgpk.pk)
}

#[instrument(err)]
pub async fn get_board_public_key_messages(board_name: &str) -> Result<Vec<Message>> {
    let mut board = get_board_client().await?;

    let valid_statements = vec![
        StatementType::Configuration,
        StatementType::ConfigurationSigned,
        StatementType::Channel,
        StatementType::ChannelsAllSigned,
        StatementType::Shares,
        StatementType::PublicKey,
        StatementType::PublicKeySigned,
    ];

    let board_messages = board.get_messages(board_name, -1).await?;
    let messages = convert_board_messages(&board_messages)?;

    let filtered_messages: Vec<Message> = messages
        .into_iter()
        .filter(|message| valid_statements.contains(&message.statement.get_kind()))
        .collect();

    Ok(filtered_messages)
}

#[instrument(err)]
pub async fn get_trustee_encrypted_private_key<C: Ctx>(
    board_name: &str,
    trustee_pub_key: &StrandSignaturePk,
) -> Result<EncryptionData> {
    let mut board = get_board_client().await?;

    let messages = board.get_messages(board_name, -1).await?;
    let pks_message = messages
        .into_iter()
        .map(|message| Message::strand_deserialize(&message.message))
        .filter_map(|message| message.ok())
        .find(|message| {
            message.statement.get_kind() == StatementType::Channel
                && message.sender.pk == *trustee_pub_key
        })
        .with_context(|| format!("Private Key not found on board {}", board_name))?;

    let bytes = pks_message.artifact.with_context(|| {
        format!(
            "Artifact missing on Private Key message on board {}",
            board_name
        )
    })?;
    let channel = Channel::<C>::strand_deserialize(&bytes).unwrap();
    Ok(channel.encrypted_channel_sk)
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

pub fn generate_trustee_set<C: Ctx>(
    configuration: &Configuration<C>,
    trustee_pks: Vec<StrandSignaturePk>,
) -> TrusteeSet {
    let mut selected_trustees: TrusteeSet = [NULL_TRUSTEE; MAX_TRUSTEES];
    let trustee_ids: Vec<usize> = trustee_pks
        .into_iter()
        .map(|trustee_pk| {
            let position = configuration
                .trustees
                .clone()
                .into_iter()
                .position(|trustee| trustee == trustee_pk);
            match position {
                Some(value) => value + 1,
                None => NULL_TRUSTEE,
            }
        })
        .collect();
    for i in 0..trustee_ids.len() {
        selected_trustees[i] = trustee_ids[i];
    }
    event!(Level::INFO, "TrusteeSet: {:?}", selected_trustees);
    selected_trustees
}

pub fn convert_board_messages(board_messages: &Vec<BoardMessage>) -> Result<Vec<Message>> {
    let messages: Vec<Message> = board_messages
        .iter()
        .map(|board_message| Message::strand_deserialize(&board_message.message))
        .collect::<Result<Vec<_>, StrandError>>()?;
    Ok(messages)
}

pub async fn get_protocol_manager<C: Ctx>(board_name: &str) -> Result<ProtocolManager<C>> {
    let protocol_manager_key = get_protocol_manager_secret_path(board_name);
    let protocol_manager_data = vault::read_secret(protocol_manager_key)
        .await?
        .ok_or(anyhow!("protocol manager secret not found"))?;
    Ok(deserialize_protocol_manager::<C>(protocol_manager_data))
}

pub async fn get_board_messages<C: Ctx>(
    board_name: &str,
    board: &mut BoardClient,
) -> Result<Vec<Message>> {
    let pm = get_protocol_manager::<C>(board_name).await?;

    let board_messages = board.get_messages(board_name, -1).await?;
    let messages: Vec<Message> = convert_board_messages(&board_messages)?;
    Ok(messages)
}

#[instrument(
    skip(messages, configuration, public_key_hash, selected_trustees, ballots),
    err
)]
pub async fn add_ballots_to_board<C: Ctx>(
    pm: &ProtocolManager<C>,
    board: &mut BoardClient,
    board_name: &str,
    messages: &Vec<Message>,
    configuration: &Configuration<C>,
    public_key_hash: PublicKeyHash,
    selected_trustees: TrusteeSet,
    ballots: Vec<Ciphertext<C>>,
    batch: BatchNumber,
) -> Result<()> {
    let existing_message = messages.iter().find(|message| {
        let batch_number = message.statement.get_batch_number();
        let kind = message.statement.get_kind();
        batch_number == batch && StatementType::Ballots == kind
    });
    if let Some(_message) = existing_message {
        event!(
            Level::INFO,
            "Not adding Ballot to board {} as it already exists for batch {}",
            board_name,
            batch
        );
        return Ok(());
    }

    let message = Message::ballots_msg::<C, ProtocolManager<C>>(
        configuration,
        batch,
        &Ballots::<C>::new(ballots),
        selected_trustees,
        public_key_hash,
        pm,
    )?;
    info!("Adding configuration to the board..");
    let board_message: BoardMessage = message.try_into()?;
    board
        .insert_messages(board_name, &vec![board_message])
        .await
}

#[instrument(err)]
pub async fn get_board_client() -> Result<BoardClient> {
    let username = env::var("IMMUDB_USER").context("IMMUDB_USER must be set")?;
    let password = env::var("IMMUDB_PASSWORD").context("IMMUDB_PASSWORD must be set")?;
    let server_url = env::var("IMMUDB_SERVER_URL").context("IMMUDB_SERVER_URL must be set")?;

    let mut board_client = BoardClient::new(&server_url, &username, &password).await?;

    Ok(board_client)
}

#[instrument(err)]
pub async fn get_immudb_client() -> Result<Client> {
    let username = env::var("IMMUDB_USER").context("IMMUDB_USER must be set")?;
    let password = env::var("IMMUDB_PASSWORD").context("IMMUDB_PASSWORD must be set")?;
    let server_url = env::var("IMMUDB_SERVER_URL").context("IMMUDB_SERVER_URL must be set")?;

    let mut client = Client::new(&server_url, &username, &password).await?;
    client.login().await?;

    Ok(client)
}

pub fn create_named_param(name: String, value: Value) -> NamedParam {
    NamedParam {
        name,
        value: Some(SqlValue { value: Some(value) }),
    }
}
