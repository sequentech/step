// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The immudb electoral log's board: its clients, the database names the
//! platform derives for an election event and its elections, and the manager
//! key that signs the log's statements.
//!
//! The electoral log still runs on the old core (`electoral-log`, strand), so
//! the manager key kept here is the old core's `ProtocolManagerConfig`. The
//! keys ceremony and its boards use `crate::services::protocol_manager`;
//! this module goes when the electoral log moves to `cryptography`.

use anyhow::{anyhow, Context, Result};
use b4::messages::protocol_manager::{ProtocolManager, ProtocolManagerConfig};
use deadpool_postgres::Transaction;
use electoral_log::BoardClient;
use immudb_rs::{sql_value::Value, Client, NamedParam, SqlValue};
use std::env;
use std::marker::PhantomData;
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::signature::StrandSignatureSk;
use tracing::instrument;

use crate::services::protocol_manager::get_protocol_manager_secret_path;
use crate::services::vault;

pub fn get_event_board(tenant_id: &str, election_event_id: &str, slug: &str) -> String {
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

pub fn get_election_board(tenant_id: &str, election_id: &str, slug: &str) -> String {
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

#[instrument(err)]
pub async fn get_board_client() -> Result<BoardClient> {
    let username = env::var("IMMUDB_USER").context("IMMUDB_USER must be set")?;
    let password = env::var("IMMUDB_PASSWORD").context("IMMUDB_PASSWORD must be set")?;
    let server_url = env::var("IMMUDB_SERVER_URL").context("IMMUDB_SERVER_URL must be set")?;

    let board_client = BoardClient::new(&server_url, &username, &password).await?;

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
    let pmkey: StrandSignatureSk =
        StrandSignatureSk::generate().map_err(|err| anyhow!("{:?}", err))?;
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
