// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The client of the old core's b4 board (`packages/b4`, Postgres backed) and
//! the helpers the tally still needs on it. The keys ceremony runs on the new
//! core (`crate::services::protocol_board`); this module goes away with
//! `packages/b4` once the tally is migrated too.

use anyhow::{anyhow, Context, Result};
use b4::client::pgsql::{B3MessageRow, PgsqlB3Client, PgsqlConnectionParams};
use b4::messages::artifact::{Ballots, Configuration, DkgPublicKey};
use b4::messages::message::Message;
use b4::messages::newtypes::{BatchNumber, PublicKeyHash, TrusteeSet, MAX_TRUSTEES, NULL_TRUSTEE};
use b4::messages::protocol_manager::ProtocolManager;
use b4::messages::statement::StatementType;
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::KeysCeremony;
use std::env;
use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::serialization::{StrandDeserialize, StrandSerialize};
use strand::signature::StrandSignaturePk;
use strand::util::StrandError;
use tracing::{event, info, instrument, Level};

use crate::postgres::election::get_elections_by_keys_ceremony_id;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log_board::get_election_board;

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

// returns (board_name, election_id), where the election_id might be None for an event Board
#[instrument(skip(transaction), err)]
pub async fn get_keys_ceremony_board(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    keys_ceremony: &KeysCeremony,
) -> Result<(String, Option<String>)> {
    if keys_ceremony.is_default() {
        // fetch election_event
        let election_event =
            get_election_event_by_id(transaction, tenant_id, election_event_id).await?;

        // get board name
        let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
            .with_context(|| "missing bulletin board")?;
        Ok((board_name, None))
    } else {
        let election = get_elections_by_keys_ceremony_id(
            transaction,
            tenant_id,
            election_event_id,
            &keys_ceremony.id,
        )
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| {
            anyhow!(
                "Can't find election with keys ceremony {}",
                keys_ceremony.id
            )
        })?;
        let slug = std::env::var("ENV_SLUG").with_context(|| "missing env var ENV_SLUG")?;
        let board = get_election_board(tenant_id, &election.id, &slug);
        Ok((board, Some(election.id)))
    }
}

#[instrument(err)]
pub fn deserialize_public_key(public_key_string: String) -> Result<StrandSignaturePk> {
    StrandSignaturePk::from_der_b64_string(&public_key_string).map_err(|err| anyhow!("{:?}", err))
}

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
pub fn convert_board_messages(board_messages: &Vec<B3MessageRow>) -> Result<Vec<Message>> {
    board_messages
        .iter()
        .map(|board_message| Message::strand_deserialize(&board_message.message))
        .collect::<Result<Vec<_>, StrandError>>()
        .map_err(Into::into)
}

pub async fn get_board_messages<C: Ctx>(
    board_name: &str,
    b3_client: &PgsqlB3Client,
) -> Result<Vec<Message>> {
    let board_messages = b3_client.get_messages(board_name, -1).await?;
    let messages: Vec<Message> = convert_board_messages(&board_messages)?;
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
