// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::keycloak::get_keyckloak_pool;
use crate::utils::read_config::load_external_config;
use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose;
use base64::Engine;
use clap::Args;
use colored::Colorize;
use csv::WriterBuilder;
use electoral_log::domain::{
    Filter, LogQuery, NumberColumn, NumberComparison, OrderColumn, SortDirection, SqlCompOperators,
    TextColumn,
};
use electoral_log::messages::message::Message;
use electoral_log::messages::newtypes::{CastVoteHash, ElectionIdString, PseudonymHash};
use electoral_log::messages::statement::{StatementBody, StatementType};
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::encrypt::shorten_hash;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use strand::serialization::StrandDeserialize;
use tokio_postgres::Transaction;
use uuid::Uuid;
use windmill::services::partial_file::PartialFile;
use windmill::services::protocol_manager::get_board_client;
use windmill::services::providers::transactions_provider::provide_hasura_transaction;
#[derive(Serialize)]
struct Record {
    created: i64,
    election_id: ElectionIdString,
    area_id: Option<String>,
    hash_voter_id: String,
    ballot_id: String,
    voting_channel: String,
}

struct CastVoteExportFields<'a> {
    election_id: &'a ElectionIdString,
    pseudonym_hash: &'a PseudonymHash,
    cast_vote_hash: &'a CastVoteHash,
    voting_channel: String,
}

fn cast_vote_export_fields(body: &StatementBody) -> Option<CastVoteExportFields<'_>> {
    match body {
        StatementBody::CastVote(election_id, pseudonym, cast_vote, _, _) => {
            Some(CastVoteExportFields {
                election_id,
                pseudonym_hash: pseudonym,
                cast_vote_hash: cast_vote,
                voting_channel: VotingStatusChannel::ONLINE.to_string(),
            })
        }
        StatementBody::CastVoteWithChannel(election_id, pseudonym, cast_vote, _, _, channel) => {
            Some(CastVoteExportFields {
                election_id,
                pseudonym_hash: pseudonym,
                cast_vote_hash: cast_vote,
                voting_channel: channel.0.clone(),
            })
        }
        _ => None,
    }
}

#[derive(Args)]
#[command(about = "Export casted a vote", long_about = None)]
pub struct ExportCastVotes {
    /// Electoral-log board name; PostgreSQL connection uses ELECTORAL_LOG_PG_*
    #[arg(long)]
    board_db: String,

    // Filename: Name of the output file
    #[arg(long, default_value = "output.csv")]
    output: String,
}

impl ExportCastVotes {
    pub fn run(&self) -> Result<()> {
        let runtime = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;
        let output = runtime
            .block_on(self.run_export_cast_votes())
            .context("Failed to export cast votes")?;
        println!(
            "{} {}",
            "Successfully exported cast votes to".green(),
            output.display()
        );
        Ok(())
    }

    /// Writes the export as `<output>.partial` and renames it to `<output>` only
    /// once every cast vote is written, so a failed export leaves no file that
    /// looks complete.
    pub async fn run_export_cast_votes(&self) -> Result<PathBuf> {
        let file = PartialFile::create(&self.output)?;
        println!("Writing {}", file.path().display());
        let mut writer = WriterBuilder::new().from_writer(file);

        let client = get_board_client().await?;
        let mut last_id = 0;
        loop {
            let query = LogQuery {
                filters: vec![
                    Filter::Text(
                        TextColumn::StatementKind,
                        SqlCompOperators::Equal,
                        StatementType::CastVote.to_string(),
                    ),
                    Filter::Number(NumberColumn::Id, NumberComparison::GreaterThan, last_id),
                ],
                order: vec![(OrderColumn::Id, SortDirection::Asc)],
                limit: 1000,
                ..LogQuery::default()
            };
            let electoral_log_messages = client
                .query(&self.board_db, &query)
                .await
                .context("Failed to read cast votes from the electoral log")?;
            let Some(last) = electoral_log_messages.last() else {
                break;
            };
            last_id = last.id;
            println!("Parsing {} messages", electoral_log_messages.len());
            for electoral_log_message in electoral_log_messages {
                let message: &Message =
                    &Message::strand_deserialize(&electoral_log_message.message)
                        .map_err(|err| anyhow!("Failed to deserialize message: {:?}", err))?;

                let Some(fields) = cast_vote_export_fields(&message.statement.body) else {
                    continue;
                };

                writer
                    .serialize(Record {
                        created: electoral_log_message.created,
                        election_id: fields.election_id.clone(),
                        hash_voter_id: hex::encode(fields.pseudonym_hash.0.clone().to_inner()),
                        ballot_id: hex::encode(shorten_hash(
                            &fields.cast_vote_hash.0.clone().to_inner(),
                        )),
                        area_id: electoral_log_message.area_id.clone(),
                        voting_channel: fields.voting_channel,
                    })
                    .map_err(|error| anyhow!("Failed to write row {}", error))?;
            }
        }
        writer
            .into_inner()
            .map_err(|error| anyhow!("Failed to write {}: {}", self.output, error.error()))?
            .commit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use electoral_log::messages::newtypes::{
        VoterCountryString, VoterIpString, VotingChannelString,
    };

    fn cast_vote_body() -> StatementBody {
        StatementBody::CastVote(
            ElectionIdString(Some("election-id".to_string())),
            PseudonymHash::new([1; 64]),
            CastVoteHash::new([2; 64]),
            VoterIpString("ip".to_string()),
            VoterCountryString("country".to_string()),
        )
    }

    #[test]
    fn a_failed_export_is_an_error_and_leaves_no_file() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("missing").join("votes.csv");
        let command = ExportCastVotes {
            board_db: "board".to_string(),
            output: output.display().to_string(),
        };

        let error = format!("{:#}", command.run().unwrap_err());

        assert!(error.contains("Failed to export cast votes"), "{error}");
        assert!(error.contains("votes.csv.partial"), "{error}");
        assert!(!output.exists());
    }

    #[test]
    fn legacy_cast_votes_export_as_online() {
        let body = cast_vote_body();
        let fields = cast_vote_export_fields(&body).unwrap();

        assert_eq!(fields.voting_channel, "ONLINE");
        assert_eq!(fields.election_id.0.as_deref(), Some("election-id"));
    }

    #[test]
    fn channel_aware_cast_votes_export_the_stored_channel() {
        let body = StatementBody::CastVoteWithChannel(
            ElectionIdString(Some("election-id".to_string())),
            PseudonymHash::new([1; 64]),
            CastVoteHash::new([2; 64]),
            VoterIpString("ip".to_string()),
            VoterCountryString("country".to_string()),
            VotingChannelString("TELEPHONE".to_string()),
        );
        let fields = cast_vote_export_fields(&body).unwrap();

        assert_eq!(fields.voting_channel, "TELEPHONE");
        assert_eq!(fields.election_id.0.as_deref(), Some("election-id"));
    }
}
