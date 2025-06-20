// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::keycloak::get_keyckloak_pool;
use crate::utils::read_config::load_external_config;
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose;
use base64::Engine;
use clap::Args;
use csv::WriterBuilder;
use electoral_log::client::types::{
    ElectoralLogVarCharColumn, SqlCompOperators, WhereClauseOrdMap,
};
use electoral_log::messages::message::Message;
use electoral_log::messages::newtypes::ElectionIdString;
use electoral_log::messages::statement::{StatementBody, StatementType};
use electoral_log::BoardClient;
use sequent_core::encrypt::shorten_hash;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use strand::serialization::StrandDeserialize;
use tokio_postgres::Transaction;
use uuid::Uuid;
use windmill::services::providers::transactions_provider::provide_hasura_transaction;
#[derive(Serialize)]
struct Record {
    created: i64,
    election_id: ElectionIdString,
    area_id: Option<String>,
    hash_voter_id: String,
    ballot_id: String,
}

#[derive(Args)]
#[command(about = "Export casted a vote", long_about = None)]
pub struct ExportCastVotes {
    /// Server url - Url for connecting to immudb board
    #[arg(long)]
    server_url: String,

    /// Username - Username to connect to immudb
    #[arg(long)]
    username: String,

    /// Password - Password to connect to immudb
    #[arg(long)]
    password: String,

    /// Board DB - Immudb Board name
    #[arg(long)]
    board_db: String,

    // Filename: Name of the output file
    #[arg(long, default_value = "output.csv")]
    output: String,
}

impl ExportCastVotes {
    pub fn run(&self) {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        match runtime.block_on(self.run_export_cast_votes()) {
            Ok(_) => println!("Successfully exported cast votes"),
            Err(err) => eprintln!("Error! Failed to export cast votes: {err:?}"),
        }
    }

    pub async fn run_export_cast_votes(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Creating file {}", self.output);
        let file = File::create(&self.output)?;

        println!("Creating writer");
        let mut writer = WriterBuilder::new().from_writer(&file);

        println!("Creating client");
        let mut client = BoardClient::new(&self.server_url, &self.username, &self.password)
            .await
            .map_err(|err| anyhow!("Failed to create the client: {:?}", err))?;

        let cols_match = WhereClauseOrdMap::from(&[(
            ElectoralLogVarCharColumn::StatementKind,
            SqlCompOperators::Equal(StatementType::CastVote.to_string()),
        )]);
        let order_by: Option<HashMap<String, String>> = None;
        println!("Getting messages");
        let electoral_log_messages = client
            .get_electoral_log_messages_filtered(
                &self.board_db,
                Some(cols_match),
                None,
                None,
                None,
                None,
                order_by,
            )
            .await
            .map_err(|err| anyhow!("Failed to get filtered messages: {:?}", err))?;

        println!("Parsing {} messages", electoral_log_messages.len());
        for electoral_log_message in electoral_log_messages {
            let message: &Message = &Message::strand_deserialize(&electoral_log_message.message)
                .map_err(|err| anyhow!("Failed to deserialize message: {:?}", err))?;

            if let StatementBody::CastVote(
                election_id_string,
                pseudonym_hash,
                cast_vote_hash,
                _voter_ip,
                _voter_country,
            ) = &message.statement.body
            {
                writer
                    .serialize(Record {
                        created: electoral_log_message.created,
                        election_id: election_id_string.clone(),
                        hash_voter_id: hex::encode(pseudonym_hash.0.clone().to_inner()),
                        ballot_id: hex::encode(shorten_hash(&cast_vote_hash.0.clone().to_inner())),
                        area_id: electoral_log_message.area_id.clone(),
                    })
                    .map_err(|error| anyhow!("Failed to write row {}", error))?;
            };
        }

        writer
            .flush()
            .map_err(|error| anyhow!("Failed to flush writer {}", error))?;

        Ok(())
    }
}
