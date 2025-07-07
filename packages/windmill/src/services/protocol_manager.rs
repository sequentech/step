// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use b3::client::pgsql::{PgsqlB3Client, PgsqlConnectionParams};
use b3::messages::artifact::Shares;
use b3::messages::artifact::{Ballots, Channel, Configuration, DkgPublicKey, TrusteeShareData};
use b3::messages::message::Message;
use b3::messages::newtypes::BatchNumber;
use b3::messages::newtypes::PublicKeyHash;
use b3::messages::newtypes::{TrusteeSet, MAX_TRUSTEES, NULL_TRUSTEE};
use b3::messages::protocol_manager::{ProtocolManager, ProtocolManagerConfig};
use b3::messages::statement::StatementType;
use deadpool_postgres::Transaction;
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::StrandDeserialize;
use strand::serialization::StrandSerialize;
use strand::util::StrandError;

use anyhow::{anyhow, Context, Result};
use std::env;
use std::marker::PhantomData;
use tracing::{event, info, instrument, Level};

use crate::services::vault;
use b3::client::pgsql::B3MessageRow;
use electoral_log::BoardClient;
use immudb_rs::{sql_value::Value, Client, NamedParam, SqlValue};
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

pub fn get_protocol_manager_secret_path(board_name: &str) -> String {
    format!("boards/{board_name}/protocol-manager")
}

#[instrument(skip(hasura_transaction), err)]
pub async fn create_protocol_manager_keys(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board_name: &str,
) -> Result<()> {
    // create protocol manager keys
    let protocol_manager = gen_protocol_manager::<RistrettoCtx>()?;
    // save protocol manager keys in vault
    let protocol_config = serialize_protocol_manager::<RistrettoCtx>(&protocol_manager)?;
    let protocol_key = get_protocol_manager_secret_path(board_name);
    vault::save_secret(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        &protocol_key,
        &protocol_config,
    )
    .await?;
    Ok(())
}

#[instrument]
pub fn gen_protocol_manager<C: Ctx>() -> Result<ProtocolManager<C>> {
    let pmkey: StrandSignatureSk = StrandSignatureSk::gen().map_err(|err| anyhow!("{:?}", err))?;
    let pm: ProtocolManager<C> = ProtocolManager {
        signing_key: pmkey,
        phantom: PhantomData,
    };

    Ok(pm)
}

#[instrument]
pub fn serialize_protocol_manager<C: Ctx>(pm: &ProtocolManager<C>) -> Result<String> {
    let pmc = ProtocolManagerConfig::from(&pm);
    toml::to_string(&pmc).map_err(|err| anyhow!("{:?}", err))
}

#[instrument]
pub fn deserialize_protocol_manager<C: Ctx>(contents: String) -> Result<ProtocolManager<C>> {
    let pmc: ProtocolManagerConfig =
        toml::from_str(&contents).map_err(|err| anyhow!("{:?}", err))?;
    let pmkey = pmc.get_signing_key().map_err(|err| anyhow!("{:?}", err))?;
    Ok(ProtocolManager::new(pmkey))
}

#[instrument(err, skip_all)]
async fn init<C: Ctx>(
    b3_client: &mut PgsqlB3Client,
    configuration: Configuration<C>,
    pm: ProtocolManager<C>,
    board_name: &str,
) -> Result<()> {
    let message = Message::bootstrap_msg(&configuration, &pm)?;
    info!("Adding configuration to the board..");
    b3_client
        .insert_configuration::<C>(board_name, message)
        .await
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

    // let mut board_client = get_board_client().await?;
    let mut client = get_b3_pgsql_client().await?;

    init(&mut client, configuration, pm, board_name).await
}

#[instrument(err)]
pub async fn get_board_public_key<C: Ctx>(board_name: &str) -> Result<C::E> {
    let mut board = get_b3_pgsql_client().await?;

    let b3 = board.get_messages(board_name, -1).await?;

    let valid_statements = vec![StatementType::PublicKey, StatementType::PublicKeySigned];
    let messages: Vec<Message> = b3
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
    let dkgpk =
        DkgPublicKey::<C>::strand_deserialize(&bytes).map_err(|err| anyhow!("{:?}", err))?;
    Ok(dkgpk.pk)
}

pub async fn check_configuration_exists(board_name: &str) -> Result<bool> {
    let board = get_b3_pgsql_client().await?;

    let b3 = board.get_messages(board_name, -1).await?;
    let messages = convert_b3(&b3)?;

    let found_config = messages
        .into_iter()
        .find(|message| StatementType::Configuration == message.statement.get_kind());
    Ok(found_config.is_some())
}

#[instrument(err)]
pub async fn get_board_public_key_messages(board_name: &str) -> Result<Vec<Message>> {
    let board = get_b3_pgsql_client().await?;

    let valid_statements = vec![
        StatementType::Configuration,
        StatementType::ConfigurationSigned,
        StatementType::Channel,
        StatementType::ChannelsAllSigned,
        StatementType::Shares,
        StatementType::PublicKey,
        StatementType::PublicKeySigned,
    ];

    let b3 = board.get_messages(board_name, -1).await?;
    let messages = convert_b3(&b3)?;

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
) -> Result<TrusteeShareData<C>> {
    let board = get_b3_pgsql_client().await?;

    // let messages = board.get_messages(board_name, -1).await?;
    let messages = board
        .get_with_kind(board_name, StatementType::Channel, trustee_pub_key)
        .await?;

    let channel_message = messages
        .into_iter()
        .map(|message| Message::strand_deserialize(&message.message))
        .filter_map(|message| message.ok())
        .next()
        .with_context(|| format!("Channel not found on board {}", board_name))?;

    let messages = board
        .get_with_kind_only(board_name, StatementType::Shares)
        .await?;

    let shares: Result<Vec<Message>> = messages
        .into_iter()
        .map(|message| Ok(Message::strand_deserialize(&message.message)?))
        .collect();

    let shares: Result<Vec<Shares<C>>> = shares?
        .into_iter()
        .map(|s| {
            let bytes = s.artifact.ok_or(anyhow!("Shares missing artifact bytes"))?;
            let shares = Shares::<C>::strand_deserialize(&bytes)?;
            Ok(shares)
        })
        .collect();

    let channel_bytes = channel_message.artifact.with_context(|| {
        format!(
            "Artifact missing on Private Key message on board {}",
            board_name
        )
    })?;
    let channel =
        Channel::<C>::strand_deserialize(&channel_bytes).map_err(|err| anyhow!("{:?}", err))?;

    let ret = TrusteeShareData {
        channel,
        shares: shares?,
    };

    Ok(ret)

    // Ok(channel.encrypted_channel_sk)
}

#[instrument(skip_all, err)]
pub fn get_configuration<C: Ctx>(messages: &Vec<Message>) -> Result<Configuration<C>> {
    let configuration_msg = messages
        .iter()
        .find(|message| {
            StatementType::Configuration == message.statement.get_kind()
                && message.artifact.is_some()
        })
        .ok_or(anyhow!("Can't find configuration message"))?;
    Ok(Configuration::<C>::strand_deserialize(
        &configuration_msg
            .artifact
            .clone()
            .ok_or(anyhow!("Missing artifact on configuration message"))?,
    )?)
}

#[instrument(skip_all, err)]
pub fn get_public_key_hash<C: Ctx>(messages: &Vec<Message>) -> Result<PublicKeyHash> {
    let public_key_message = messages
        .iter()
        .find(|message| {
            StatementType::PublicKey == message.statement.get_kind() && message.artifact.is_some()
        })
        .ok_or(anyhow!("Can't find public key message"))?;
    let public_key_bytes = public_key_message
        .artifact
        .clone()
        .ok_or(anyhow!("Public key message artifact missing"))?;
    let dkgpk = DkgPublicKey::<C>::strand_deserialize(&public_key_bytes)?;
    let pk_bytes = dkgpk.strand_serialize()?;
    let pk_h = strand::hash::hash_to_array(&pk_bytes)?;
    Ok(PublicKeyHash(strand::util::to_u8_array(&pk_h)?))
}

#[instrument(skip_all)]
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

#[instrument(skip_all, err)]
pub fn convert_b3(b3: &Vec<B3MessageRow>) -> Result<Vec<Message>> {
    let messages: Vec<Message> = b3
        .iter()
        .map(|board_message| Message::strand_deserialize(&board_message.message))
        .collect::<Result<Vec<_>, StrandError>>()?;
    Ok(messages)
}

#[instrument(err)]
pub async fn get_protocol_manager<C: Ctx>(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    board_name: &str,
) -> Result<ProtocolManager<C>> {
    let protocol_manager_key = get_protocol_manager_secret_path(board_name);
    let protocol_manager_data = vault::read_secret(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &protocol_manager_key,
    )
    .await?
    .ok_or(anyhow!("protocol manager secret not found"))?;
    deserialize_protocol_manager::<C>(protocol_manager_data)
}

#[instrument(skip(b3_client), err)]
pub async fn get_b3<C: Ctx>(
    board_name: &str,
    b3_client: &mut PgsqlB3Client,
) -> Result<Vec<Message>> {
    let b3 = b3_client.get_messages(board_name, -1).await?;
    let messages: Vec<Message> = convert_b3(&b3)?;
    Ok(messages)
}

#[instrument(
    skip(
        messages,
        configuration,
        public_key_hash,
        selected_trustees,
        ballots,
        b3_client
    ),
    err
)]
pub async fn add_ballots_to_board<C: Ctx>(
    pm: &ProtocolManager<C>,
    b3_client: &mut PgsqlB3Client,
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

    let ballots_len = ballots.len();

    let message = Message::ballots_msg::<C, ProtocolManager<C>>(
        configuration,
        batch,
        &Ballots::<C>::new(ballots),
        selected_trustees,
        public_key_hash,
        pm,
    )?;
    info!(
        "Adding configuration to the board for batch {} and number of ballots {}",
        batch, ballots_len
    );
    b3_client.insert_ballots::<C>(board_name, message).await
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
pub async fn get_b3_pgsql_client() -> Result<PgsqlB3Client> {
    let username = env::var("B3_PG_USER").context("B3_PG_USER must be set")?;
    let password = env::var("B3_PG_PASSWORD").context("B3_PG_PASSWORD must be set")?;
    let host = env::var("B3_PG_HOST").context("B3_PG_HOST must be set")?;
    let port = env::var("B3_PG_PORT").context("B3_PG_PORT must be set")?;
    let database = env::var("B3_PG_DATABASE").context("B3_PG_DATABASE must be set")?;

    let port: u32 = port.parse::<u32>()?;

    let c = PgsqlConnectionParams::new(&host, port, &username, &password);
    let c_db = c.with_database(&database);
    let client = PgsqlB3Client::new(&c_db).await?;

    Ok(client)
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

pub fn get_event_board(tenant_id: &str, election_event_id: &str) -> String {
    let slug = std::env::var("ENV_SLUG").unwra();
    let tenant: String = tenant_id
        .to_string()
        .chars()
        .filter(|&c| c != '-')
        .take(17)
        .collect();
    format!("{}tenant{}event{}", slug, tenant, election_event_id)
        .chars()
        .filter(|&c| c != '-')
        .collect()
}

pub fn get_election_board(tenant_id: &str, election_id: &str) -> String {
    let slug = std::env::var("ENV_SLUG").unwrap();
    let tenant: String = tenant_id
        .to_string()
        .chars()
        .filter(|&c| c != '-')
        .take(17)
        .collect();
    format!("{}tenant{}election{}", slug, tenant, election_id)
        .chars()
        .filter(|&c| c != '-')
        .collect()
}

pub fn convert_board_messages(board_messages: &Vec<B3MessageRow>) -> Result<Vec<Message>> {
    let messages: Vec<Message> = board_messages
        .iter()
        .map(|m| Message::strand_deserialize(&m.message))
        .collect::<Result<Vec<_>, StrandError>>()?;
    Ok(messages)
}

pub async fn get_board_messages<C: Ctx>(
    board_name: &str,
    b3_client: &PgsqlB3Client,
) -> Result<Vec<Message>> {
    let board_messages = b3_client.get_messages(board_name, -1).await?;
    let messages: Vec<Message> = convert_board_messages(&board_messages)?;
    Ok(messages)
}
