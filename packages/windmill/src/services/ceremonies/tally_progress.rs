// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use b3::messages::{artifact::Plaintexts, message::Message, statement::StatementType};
use sequent_core::types::{
    ceremonies::{TallyElection, TallyElectionStatus},
    hasura::core::{TallySession, TallySessionContest},
};
use std::collections::{HashMap, HashSet};
use tracing::{event, instrument, Level};

#[instrument(skip_all)]
fn get_session_ids_by_type(messages: &Vec<Message>, kind: StatementType) -> Vec<i64> {
    let mut plaintext_batch_ids: Vec<i64> = messages
        .iter()
        .map(|message| {
            if kind == message.statement.get_kind() {
                message.statement.get_batch_number() as i64
            } else {
                -1i64
            }
        })
        .filter(|value| *value > -1)
        .collect();
    plaintext_batch_ids.sort_by_key(|id| id.clone());
    plaintext_batch_ids.dedup();
    plaintext_batch_ids
}

#[instrument(skip_all, ret)]
pub async fn generate_tally_progress(
    tally_session: TallySession,
    tally_session_contest: Vec<TallySessionContest>,
    messages: &Vec<Message>,
) -> Result<Vec<TallyElection>> {
    let mut complete_map: HashMap<String, Vec<i64>> = HashMap::new();
    // let tally_session = tally_session_data
    //     .sequent_backend_tally_session
    //     .first()
    //     .ok_or(anyhow!("Missing tally session"))?;
    let all_election_ids: HashSet<String> = tally_session
        .election_ids
        .clone()
        .unwrap_or(vec![])
        .into_iter()
        .collect();
    for contest in &tally_session_contest {
        let mut batch_ids = complete_map
            .get(&contest.election_id)
            .map(|v| v.clone())
            .unwrap_or(vec![]);
        batch_ids.push(contest.session_id as i64);
        complete_map.insert(contest.election_id.clone(), batch_ids.clone());
    }
    let finished_batch_ids: Vec<i64> = get_session_ids_by_type(messages, StatementType::Plaintexts);

    let mut decrypting_batch_ids: Vec<i64> =
        get_session_ids_by_type(messages, StatementType::DecryptionFactors);

    decrypting_batch_ids = decrypting_batch_ids
        .into_iter()
        .filter(|value| !finished_batch_ids.contains(&value))
        .collect();

    let mut mixing_batch_ids: Vec<i64> = get_session_ids_by_type(messages, StatementType::Mix);

    mixing_batch_ids = mixing_batch_ids
        .into_iter()
        .filter(|value| {
            !finished_batch_ids.contains(&value) && !decrypting_batch_ids.contains(&value)
        })
        .collect();

    let mut tally_elections_status: Vec<TallyElection> = complete_map
        .iter()
        .map(|(key, election_batch_ids)| {
            let num_finished_contests = finished_batch_ids
                .iter()
                .filter(|value| election_batch_ids.contains(value))
                .collect::<Vec<_>>()
                .len();
            let num_decrypting_contests = decrypting_batch_ids
                .iter()
                .filter(|value| election_batch_ids.contains(value))
                .collect::<Vec<_>>()
                .len();
            let num_mixing_contests = mixing_batch_ids
                .iter()
                .filter(|value| election_batch_ids.contains(value))
                .collect::<Vec<_>>()
                .len();
            let total = election_batch_ids.len();
            let mut progress: f64 = 100.0
                * (0.2 * (num_mixing_contests as f64)
                    + 0.4 * (num_decrypting_contests as f64)
                    + (num_finished_contests as f64))
                / (total as f64);
            // clamp values to 0-100
            progress = progress.min(100.0).max(0.0);
            let new_status = if num_finished_contests >= total {
                TallyElectionStatus::SUCCESS
            } else if num_decrypting_contests == 0 && num_mixing_contests > 0 {
                TallyElectionStatus::MIXING
            } else if num_decrypting_contests > 0 {
                TallyElectionStatus::DECRYPTING
            } else {
                TallyElectionStatus::WAITING
            };

            TallyElection {
                election_id: key.clone(),
                status: new_status,
                progress,
            }
        })
        .collect();
    // take into consideration that some elections might not have an assigned area
    let existing_election_ids: HashSet<String> = complete_map.into_keys().collect();
    let missing_elements: Vec<String> = all_election_ids
        .difference(&existing_election_ids)
        .cloned()
        .collect();
    for missing_election_id in missing_elements {
        tally_elections_status.push(TallyElection {
            election_id: missing_election_id.clone(),
            status: TallyElectionStatus::SUCCESS,
            progress: 100.0,
        });
    }
    tally_elections_status.sort_by_key(|status| status.election_id.clone());
    Ok(tally_elections_status)
}
