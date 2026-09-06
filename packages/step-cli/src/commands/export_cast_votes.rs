// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use clap::Args;
use colored::Colorize;
use csv::WriterBuilder;
use electoral_log::messages::message::Message;
use electoral_log::messages::newtypes::{CastVoteHash, ElectionIdString, PseudonymHash};
use electoral_log::messages::statement::{StatementBody, StatementType};
use electoral_log::{BoardClient, ElectoralLogVarCharColumn, SqlCompOperators};
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::encrypt::shorten_hash;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs::File;
use strand::serialization::StrandDeserialize;
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
            Ok(_) => println!("{}", "Successfully exported cast votes".green()),
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

        let cols_match = BTreeMap::from([(
            ElectoralLogVarCharColumn::StatementKind,
            (SqlCompOperators::Equal, StatementType::CastVote.to_string()),
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

        writer
            .flush()
            .map_err(|error| anyhow!("Failed to flush writer {}", error))?;

        Ok(())
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
