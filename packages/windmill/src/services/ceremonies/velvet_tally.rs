// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::tally_session_execution::get_last_tally_session_execution::{
    GetLastTallySessionExecutionSequentBackendTallySessionContest, ResponseData,
};
use anyhow::{anyhow, Context, Result};
use sequent_core::ballot::{BallotStyle, Contest};
use sequent_core::ballot_codec::PlaintextCodec;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use strand::{backend::ristretto::RistrettoCtx, context::Ctx, serialization::StrandDeserialize};
use tracing::{event, instrument, Level};
use uuid::{uuid, Uuid};
use velvet::cli::state::State;
use velvet::cli::CliRun;
use velvet::fixtures::get_config;
use velvet::pipes::pipe_inputs::{AreaConfig, ElectionConfig};

pub type AreaContestDataType = (
    Vec<<RistrettoCtx as Ctx>::P>,
    GetLastTallySessionExecutionSequentBackendTallySessionContest,
    Contest,
    BallotStyle,
);

#[instrument(skip_all)]
fn decode_plantexts_to_biguints(
    plaintexts: &Vec<<RistrettoCtx as Ctx>::P>,
    contest: &Contest,
) -> Vec<String> {
    plaintexts
        .iter()
        .filter_map(|plaintext| {
            let biguint = contest.decode_plaintext_contest_to_biguint(plaintext);

            match biguint {
                Ok(v) => {
                    let biguit_str = v.to_str_radix(10);

                    Some(biguit_str)
                }
                Err(e) => {
                    event!(Level::WARN, "Decoding plaintext has failed: {e}");
                    None
                }
            }
        })
        .collect::<Vec<_>>()
}

#[instrument(skip_all, err)]
pub fn prepare_tally_for_area_contest(
    base_tempdir: PathBuf,
    area_contest_plaintext: &AreaContestDataType,
) -> Result<()> {
    let (plaintexts, tally_session_contest, contest, ballot_style) = area_contest_plaintext.clone();

    let area_id = tally_session_contest.area_id.clone();
    let contest_id = contest.id.clone();
    let election_id = contest.election_id.clone();

    let biguit_ballots = decode_plantexts_to_biguints(&plaintexts, &contest);

    let velvet_input_dir = base_tempdir.join("input");
    let velvet_output_dir = base_tempdir.join("output");

    //// create ballots
    let ballots_path = velvet_input_dir.join(format!(
        "default/ballots/election__{election_id}/contest__{contest_id}/area__{area_id}"
    ));
    fs::create_dir_all(&ballots_path)?;

    let csv_ballots_path = ballots_path.join("ballots.csv");
    let mut csv_ballots_file = File::create(&csv_ballots_path)?;
    let buffer = biguit_ballots.join("\n").into_bytes();

    csv_ballots_file.write_all(&buffer)?;

    //// create area folder
    let area_path: PathBuf = velvet_input_dir.join(format!(
        "default/configs/election__{election_id}/contest__{contest_id}/area__{area_id}"
    ));
    fs::create_dir_all(&area_path)?;
    // create area config
    let area_config_path: PathBuf = velvet_input_dir.join(format!(
        "default/configs/election__{election_id}/contest__{contest_id}/area__{area_id}/area-config.json"
    ));
    let area_config = AreaConfig {
        id: Uuid::parse_str(&area_id)?,
        tenant_id: Uuid::parse_str(&contest.tenant_id)?,
        election_event_id: Uuid::parse_str(&contest.election_event_id)?,
        election_id: Uuid::parse_str(&election_id)?,
        census: biguit_ballots.len() as u64,
    };
    let mut area_config_file = fs::File::create(area_config_path)?;
    writeln!(area_config_file, "{}", serde_json::to_string(&area_config)?)?;

    //// create contest config file
    let contest_config_path: PathBuf = velvet_input_dir.join(format!(
        "default/configs/election__{election_id}/contest__{contest_id}/contest-config.json"
    ));
    let mut contest_config_file = fs::File::create(contest_config_path)?;
    writeln!(contest_config_file, "{}", serde_json::to_string(&contest)?)?;

    Ok(())
}

#[instrument(skip_all, err)]
pub fn create_election_configs(
    base_tempdir: PathBuf,
    area_contest_plaintexts: &Vec<AreaContestDataType>,
) -> Result<()> {
    let mut elections_map: HashMap<String, ElectionConfig> = HashMap::new();

    // aggregate all ballot styles for each election
    for area_contest_plaintext in area_contest_plaintexts {
        let (plaintexts, tally_session_contest, contest, ballot_style) = area_contest_plaintext;

        let area_id = tally_session_contest.area_id.clone();
        let contest_id = contest.id.clone();
        let election_id = contest.election_id.clone();
        let mut velvet_election: ElectionConfig = match elections_map.get(&election_id) {
            Some(election) => election.clone(),
            None => ElectionConfig {
                id: Uuid::parse_str(&election_id)?,
                tenant_id: Uuid::parse_str(&contest.tenant_id)?,
                election_event_id: Uuid::parse_str(&contest.election_event_id)?,
                ballot_styles: vec![],
            },
        };
        velvet_election.ballot_styles.push(ballot_style.clone());
        elections_map.insert(election_id.clone(), velvet_election);
    }

    // deduplicate the ballot styles
    for (key, value) in &elections_map {
        let mut velvet_election: ElectionConfig = value.clone();
        velvet_election
            .ballot_styles
            .sort_by_key(|ballot_style| ballot_style.id.clone());
        velvet_election
            .ballot_styles
            .dedup_by_key(|ballot_style| ballot_style.id.clone());
    }

    // write the election configs
    for (election_id, election) in &elections_map {
        let election_config_path: PathBuf = base_tempdir.join(format!(
            "input/default/configs/election__{election_id}/election-config.json"
        ));
        let mut election_config_file = fs::File::create(election_config_path)?;
        writeln!(
            election_config_file,
            "{}",
            serde_json::to_string(&election)?
        )?;
    }

    Ok(())
}

#[instrument(err)]
pub fn call_velvet(base_tally_path: PathBuf) -> Result<State> {
    //// Run Velvet
    let cli = CliRun {
        stage: "main".to_string(),
        pipe_id: "decode-ballots".to_string(),
        config: base_tally_path.join("velvet-config.json"),
        input_dir: base_tally_path.join("input"),
        output_dir: base_tally_path.join("output"),
    };

    let config = cli.validate()?;

    let mut state = State::new(&cli, &config)?;

    while let Some(next_stage) = state.get_next() {
        let stage_name = next_stage.to_string();
        event!(Level::INFO, "Exec {}", stage_name);
        state.exec_next()?;
    }
    Ok(state)
}

pub fn create_config_file(base_tally_path: PathBuf) -> Result<()> {
    let config_path = base_tally_path.join("velvet-config.json");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&config_path)?;

    writeln!(file, "{}", serde_json::to_string(&get_config())?)?;
    Ok(())
}

#[instrument(skip(area_contest_plaintexts), err)]
pub fn run_velvet_tally(
    base_tally_path: PathBuf,
    area_contest_plaintexts: &Vec<AreaContestDataType>,
) -> Result<State> {
    for area_contest_plaintext in area_contest_plaintexts {
        prepare_tally_for_area_contest(base_tally_path.clone(), area_contest_plaintext)?;
    }
    create_election_configs(base_tally_path.clone(), area_contest_plaintexts)?;
    create_config_file(base_tally_path.clone())?;
    call_velvet(base_tally_path.clone())
}
