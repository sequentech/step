// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::keycloak::get_keyckloak_pool;
use crate::utils::read_config::load_external_config;
use anyhow::{anyhow, bail, ensure, Context, Result};
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
use tokio_postgres::Transaction;
use uuid::Uuid;
use voting_crypto::RecordDeserialize;
use windmill::services::electoral_log_board::get_board_client;
use windmill::services::partial_file::PartialFile;
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

struct CastVoteExportFields {
    election_id: ElectionIdString,
    pseudonym_hash: String,
    ballot_id: String,
    voting_channel: String,
}

fn cast_vote_export_fields(body: &StatementBody) -> Result<CastVoteExportFields> {
    let legacy = |election: &ElectionIdString,
                  pseudonym: &PseudonymHash,
                  ballot: &CastVoteHash,
                  channel: String| {
        CastVoteExportFields {
            election_id: election.clone(),
            pseudonym_hash: hex::encode(pseudonym.0.clone().to_inner()),
            ballot_id: hex::encode(shorten_hash(&ballot.0.clone().to_inner())),
            voting_channel: channel,
        }
    };
    match body {
        StatementBody::CastVote(election, pseudonym, ballot, _, _) => Ok(legacy(
            election,
            pseudonym,
            ballot,
            VotingStatusChannel::ONLINE.to_string(),
        )),
        StatementBody::CastVoteWithChannel(election, pseudonym, ballot, _, _, channel) => {
            Ok(legacy(election, pseudonym, ballot, channel.0.clone()))
        }
        StatementBody::PrivacyCastVote(evidence) => {
            // Export the signed public projection only. Never resolve a private
            // identity, copy the receipt JSON, or reinterpret the display code as a hash.
            let hash = hex::decode(&evidence.pseudonym_hash)
                .map_err(|_| anyhow!("Invalid privacy cast pseudonym hash"))?;
            ensure!(
                hash.len() == 64 && hex::encode(&hash) == evidence.pseudonym_hash,
                "Invalid privacy cast pseudonym hash"
            );
            ensure!(
                sequent_core::ballot_receipt::normalize_ballot_id(&evidence.ballot_id).as_deref()
                    == Some(evidence.ballot_id.as_str()),
                "Invalid privacy cast Ballot ID"
            );
            for id in [
                &evidence.tenant_id,
                &evidence.election_event_id,
                &evidence.election_id,
            ] {
                ensure!(Uuid::parse_str(id).is_ok(), "Invalid privacy cast scope");
            }
            ensure!(
                evidence
                    .voting_channel
                    .parse::<VotingStatusChannel>()
                    .is_ok(),
                "Invalid privacy cast voting channel"
            );
            Ok(CastVoteExportFields {
                election_id: ElectionIdString(Some(evidence.election_id.clone())),
                pseudonym_hash: evidence.pseudonym_hash.clone(),
                ballot_id: evidence.ballot_id.clone(),
                voting_channel: evidence.voting_channel.clone(),
            })
        }
        _ => bail!("Expected a cast ballot statement"),
    }
}

fn write_cast_vote<W: std::io::Write>(
    writer: &mut csv::Writer<W>,
    created: i64,
    area_id: Option<String>,
    body: &StatementBody,
) -> Result<()> {
    let fields = cast_vote_export_fields(body)?;
    writer
        .serialize(Record {
            created,
            election_id: fields.election_id,
            area_id,
            // Historical CSV header retained; new ballots contain the pseudonym hash.
            hash_voter_id: fields.pseudonym_hash,
            ballot_id: fields.ballot_id,
            voting_channel: fields.voting_channel,
        })
        .context("Failed to write cast ballot row")
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
                    &Message::record_deserialize(&electoral_log_message.message)
                        .map_err(|err| anyhow!("Failed to deserialize message: {:?}", err))?;

                write_cast_vote(
                    &mut writer,
                    electoral_log_message.created,
                    electoral_log_message.area_id,
                    &message.statement.body,
                )?;
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

    fn privacy_cast_body() -> StatementBody {
        StatementBody::PrivacyCastVote(electoral_log::messages::privacy::BallotCastEvidence {
            tenant_id: "00000000-0000-4000-8000-000000000001".into(),
            election_event_id: "00000000-0000-4000-8000-000000000002".into(),
            election_id: "00000000-0000-4000-8000-000000000003".into(),
            pseudonym_hash: "ab".repeat(64),
            ballot_id: "7QKM-2HXD".into(),
            cast_receipt_json: "{}".into(),
            voting_channel: "KIOSK".into(),
        })
    }

    #[test]
    fn privacy_cast_votes_are_not_silently_omitted() {
        let body = privacy_cast_body();
        let fields = cast_vote_export_fields(&body).expect("privacy cast must be exported");
        assert_eq!(
            fields.election_id.0.as_deref(),
            Some("00000000-0000-4000-8000-000000000003")
        );
        assert_eq!(fields.voting_channel, "KIOSK");
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

    #[test]
    fn csv_preserves_legacy_rows_and_exports_private_display_codes_without_attribution() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("ballots.csv");
        let mut writer = WriterBuilder::new().from_path(&output).unwrap();
        write_cast_vote(&mut writer, 123, Some("area-id".into()), &cast_vote_body()).unwrap();
        let mut private = privacy_cast_body();
        if let StatementBody::PrivacyCastVote(evidence) = &mut private {
            evidence.cast_receipt_json = serde_json::json!({
                "voter_id":"synthetic-private-voter", "username":"synthetic-private-name",
                "ip":"198.51.100.73", "country":"AQ", "device":"synthetic-private-device"
            })
            .to_string();
        }
        write_cast_vote(&mut writer, 124, None, &private).unwrap();
        writer.flush().unwrap();
        let csv = std::fs::read_to_string(&output).unwrap();
        assert_eq!(csv, format!(
            "created,election_id,area_id,hash_voter_id,ballot_id,voting_channel\n123,election-id,area-id,{},{},ONLINE\n124,00000000-0000-4000-8000-000000000003,,{},7QKM-2HXD,KIOSK\n",
            "01".repeat(64), "02".repeat(32), "ab".repeat(64)
        ));
        let forbidden = directory.path().join("forbidden.json");
        std::fs::write(&forbidden, serde_json::to_vec(&serde_json::json!({
            "voter_id":["synthetic-private-voter"], "username":["synthetic-private-name"],
            "ip":["198.51.100.73", "ip"], "country":["AQ", "country"], "device":["synthetic-private-device"]
        })).unwrap()).unwrap();
        let scan = std::process::Command::new("python3")
            .arg(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../scripts/vote_privacy_audit.py"),
            )
            .arg("--forbidden")
            .arg(forbidden)
            .arg(output)
            .output()
            .unwrap();
        assert!(
            scan.status.success(),
            "CLI CSV privacy scan failed: {}",
            String::from_utf8_lossy(&scan.stdout)
        );
    }

    #[test]
    fn malformed_privacy_evidence_fails_without_committing_a_partial_export() {
        for field in ["hash", "ballot", "scope", "channel"] {
            let directory = tempfile::tempdir().unwrap();
            let output = directory.path().join("ballots.csv");
            std::fs::write(&output, "previous complete export").unwrap();
            let mut writer =
                WriterBuilder::new().from_writer(PartialFile::create(&output).unwrap());
            write_cast_vote(&mut writer, 123, None, &cast_vote_body()).unwrap();
            let mut body = privacy_cast_body();
            if let StatementBody::PrivacyCastVote(evidence) = &mut body {
                match field {
                    "hash" => evidence.pseudonym_hash = "synthetic-private-value".into(),
                    "ballot" => evidence.ballot_id = "synthetic-private-value".into(),
                    "scope" => evidence.election_id = "synthetic-private-value".into(),
                    "channel" => evidence.voting_channel = "synthetic-private-value".into(),
                    _ => unreachable!(),
                }
            }
            let error = write_cast_vote(&mut writer, 124, None, &body).unwrap_err();
            assert!(error.to_string().starts_with("Invalid privacy cast"));
            assert!(!error.to_string().contains("synthetic-private-value"));
            drop(writer);
            assert_eq!(
                std::fs::read_to_string(&output).unwrap(),
                "previous complete export"
            );
            assert!(!PartialFile::partial_path(&output).exists());
        }
    }
}
