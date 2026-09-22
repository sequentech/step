// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The client for the old core's b4 board (`packages/b4`, Postgres backed),
//! and the ceremony log lines derived from its messages.
//!
//! Still read and written by the tally, the trustees' private key
//! download/check/restore steps, the bulletin board export/import and election
//! event deletion. The DKG itself runs on the new core(`crate::services::protocol_manager`)..

use anyhow::{anyhow, Context, Result};
use b4::client::pgsql::{B3MessageRow, PgsqlB3Client, PgsqlConnectionParams};
use b4::messages::artifact::{
    Ballots, Channel, Configuration, DkgPublicKey, Shares, TrusteeShareData,
};
use b4::messages::message::Message;
use b4::messages::newtypes::{BatchNumber, PublicKeyHash, TrusteeSet, MAX_TRUSTEES, NULL_TRUSTEE};
use b4::messages::protocol_manager::ProtocolManager;
use b4::messages::statement::StatementType;
use sequent_core::services::date::ISO8601;
use sequent_core::types::ceremonies::Log;
use std::env;
use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::StrandSignaturePk;
use strand::util::StrandError;
use tracing::{event, info, instrument, Level};

use crate::services::ceremonies::serialize_logs::sort_logs;

#[instrument(err)]
pub async fn get_b3_pgsql_client() -> Result<PgsqlB3Client> {
    let username = env::var("B4_PG_USER").context("B4_PG_USER must be set")?;
    let password = env::var("B4_PG_PASSWORD").context("B4_PG_PASSWORD must be set")?;
    let host = env::var("B4_PG_HOST").context("B4_PG_HOST must be set")?;
    let port = env::var("B4_PG_PORT").context("B4_PG_PORT must be set")?;
    let database = env::var("B4_PG_DATABASE").context("B4_PG_DATABASE must be set")?;

    let port: u32 = port.parse::<u32>()?;

    let c = PgsqlConnectionParams::new(&host, port, &username, &password);
    let c_db = c.with_database(&database);
    let client = PgsqlB3Client::new(&c_db).await?;

    Ok(client)
}

/// A trustee signing key of the old core, as the DER base64 the trustee
/// table used to hold.
#[instrument(err)]
pub fn deserialize_public_key(public_key_string: String) -> Result<StrandSignaturePk> {
    StrandSignaturePk::from_der_b64_string(&public_key_string).map_err(|err| anyhow!("{:?}", err))
}

#[instrument(skip_all, err)]
pub fn convert_b3(b3: &Vec<B3MessageRow>) -> Result<Vec<Message>> {
    let messages: Vec<Message> = b3
        .iter()
        .map(|board_message| Message::strand_deserialize(&board_message.message))
        .collect::<Result<Vec<_>, StrandError>>()?;
    Ok(messages)
}

pub fn convert_board_messages(board_messages: &Vec<B3MessageRow>) -> Result<Vec<Message>> {
    let messages: Vec<Message> = board_messages
        .iter()
        .map(|m| Message::strand_deserialize(&m.message))
        .collect::<Result<Vec<_>, StrandError>>()?;
    Ok(messages)
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

pub async fn get_board_messages<C: Ctx>(
    board_name: &str,
    b3_client: &PgsqlB3Client,
) -> Result<Vec<Message>> {
    let board_messages = b3_client.get_messages(board_name, -1).await?;
    let messages: Vec<Message> = convert_board_messages(&board_messages)?;
    Ok(messages)
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

pub fn message_to_log(message: &Message) -> Log {
    let batch_number = message.statement.get_batch_number();
    let timestamp = message.statement.get_timestamp() * 1000;
    let datetime = ISO8601::timestamp_ms_utc_to_date(timestamp as i64);

    Log {
        created_date: ISO8601::to_string(&datetime),
        log_text: format!(
            "{}: Added message {} for batch {}",
            &message.sender.name,
            message.statement.get_kind().to_string(),
            batch_number
        ),
    }
}

#[instrument(skip(messages), err)]
pub fn print_messages(messages: &Vec<Message>, board_name: &str) -> Result<()> {
    let logs = messages
        .iter()
        .map(|message| message_to_log(message))
        .collect();
    let sorted_logs = sort_logs(&logs);

    event!(Level::INFO, "printing messages for board {}", board_name);
    for log in sorted_logs.iter() {
        event!(Level::INFO, "{}: {}", log.created_date, log.log_text);
    }

    Ok(())
}

#[instrument(skip(messages, batch_ids), err)]
pub fn generate_logs(
    messages: &Vec<Message>,
    next_timestamp: u64,
    batch_ids: &Vec<i64>,
) -> Result<Vec<Log>> {
    let relevant_messages: Vec<&Message> = messages
        .iter()
        .filter(|message| {
            message.statement.get_timestamp() >= next_timestamp
                && batch_ids.contains(&(message.statement.get_batch_number() as i64))
        })
        .collect();
    let logs = relevant_messages
        .iter()
        .map(|message| message_to_log(message))
        .collect();
    Ok(sort_logs(&logs))
}
