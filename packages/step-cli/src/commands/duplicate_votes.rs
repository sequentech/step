// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::utils::keycloak::get_keyckloak_pool;
use crate::utils::read_config::load_external_config;
use anyhow::{anyhow, Context, Result};
use clap::Args;
use colored::Colorize;
use electoral_log::adapters::ballot_box::{AcceptBallot, AcceptOutcome, BallotStatus};
use uuid::Uuid;
use windmill::services::ballot_box::get_allowed_votes;
use windmill::services::ballot_box_reads::get_event_ballot_box;
use windmill::services::database::get_hasura_pool;
use windmill::services::insert_cast_vote::hash_voter_id;

#[derive(Args)]
#[command(about)]
pub struct DuplicateVotes {
    /// Working directory for input/output
    #[arg(long)]
    working_directory: String,

    #[arg(long)]
    num_votes: usize,
}

/// The ballot each voter casts a copy of.
struct SourceBallot {
    election_id: String,
    area_id: String,
    format: String,
    content: String,
    voter_signature: Option<Vec<u8>>,
    ballot_hash: Vec<u8>,
    voting_channel: String,
}

impl DuplicateVotes {
    /// Execute the rendering process
    pub fn run(&self) {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        match runtime.block_on(self.run_duplicate_votes(&self.working_directory, self.num_votes)) {
            Ok(_) => println!("{}", "Successfully duplicate vote".green()),
            Err(err) => eprintln!("Error! Failed to duplicate vote: {err:?}"),
        }
    }

    /// Casts a copy of the configured ballot for each of the first `num_votes`
    /// voters of the realm, through the ballot box's own rules.
    pub async fn run_duplicate_votes(
        &self,
        working_dir: &str,
        num_votes: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = load_external_config(working_dir)?;
        let realm_name = config.realm_name;
        let tenant_id = config.tenant_id;
        let election_event_id = config.election_event_id;
        let ballot_to_clone = config.duplicate_votes.row_id_to_clone;

        let kc_client = get_keyckloak_pool()
            .await?
            .get()
            .await
            .map_err(|e| anyhow!("Error getting hasura client: {}", e.to_string()))?;

        let keycloak_query = "\
            SELECT ue.id FROM user_entity AS ue \
            JOIN realm AS r ON ue.realm_id = r.id \
            WHERE r.name = $1 LIMIT $2 OFFSET 0";

        let kc_rows = kc_client
            .query(keycloak_query, &[&realm_name, &(num_votes as i64)])
            .await?;
        let existing_user_ids: Vec<String> = kc_rows
            .iter()
            .filter_map(|row| row.get::<_, Option<String>>(0))
            .collect();
        println!("Number of existing user IDs::: {}", existing_user_ids.len());

        let accepted = cast_copies(
            &tenant_id,
            &election_event_id,
            &ballot_to_clone,
            &existing_user_ids,
        )
        .await?;

        println!("Inserted {accepted} duplicate votes.");
        Ok(())
    }
}

/// Casts a copy of a ballot of the election event's ballot box for each voter,
/// with a ballot ID of its own. Returns how many the ballot box accepted.
async fn cast_copies(
    tenant_id: &str,
    election_event_id: &str,
    ballot_to_clone: &str,
    voter_ids: &[String],
) -> Result<usize> {
    let mut hasura_client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client: {e}"))?;
    let hasura_transaction = hasura_client
        .build_transaction()
        .read_only(true)
        .start()
        .await?;
    let store = get_event_ballot_box(&hasura_transaction, tenant_id, election_event_id)
        .await?
        .store;
    let row = store
        .client()
        .await?
        .query_opt(
            "SELECT election_id::text, area_id::text, format, content, voter_signature, \
                    ballot_hash, voting_channel \
             FROM ballot_box_ballot \
             WHERE election_event_id = $1::text::uuid AND id = $2::text::uuid",
            &[&election_event_id, &ballot_to_clone],
        )
        .await
        .context("Error reading the ballot to clone")?;
    let Some(row) = row else {
        println!("No ballot found to clone.");
        return Ok(0);
    };
    let source = SourceBallot {
        election_id: row.try_get(0)?,
        area_id: row.try_get(1)?,
        format: row.try_get(2)?,
        content: row.try_get(3)?,
        voter_signature: row.try_get(4)?,
        ballot_hash: row.try_get(5)?,
        voting_channel: row.try_get(6)?,
    };
    let allowed_votes = get_allowed_votes(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        &source.election_id,
    )
    .await?;

    let mut accepted = 0;
    for voter_id in voter_ids {
        let pseudonym_hash = hash_voter_id(voter_id)?;
        let ballot_id = Uuid::new_v4().simple().to_string();
        let outcome = store
            .accept_ballot(&AcceptBallot {
                election_event_id,
                election_id: &source.election_id,
                area_id: &source.area_id,
                voter_id,
                ballot_id: &ballot_id,
                format: &source.format,
                content: &source.content,
                voter_signature: source.voter_signature.as_deref(),
                pseudonym_hash: &pseudonym_hash,
                ballot_hash: &source.ballot_hash,
                voting_channel: &source.voting_channel,
                status: BallotStatus::Valid,
                voter_ip: None,
                voter_country: None,
                username: None,
                allowed_votes,
            })
            .await?;
        match outcome {
            AcceptOutcome::Accepted { .. } => accepted += 1,
            refused => println!("Voter {voter_id}: {refused:?}"),
        }
    }
    Ok(accepted)
}
