// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura::tally_session_execution::get_last_tally_session_execution::{
    GetLastTallySessionExecutionSequentBackendTallySessionContest, ResponseData,
};
use anyhow::{anyhow, Context, Result};
use sequent_core::ballot::{BallotStyle, Contest};
use sequent_core::ballot_codec::PlaintextCodec;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use strand::{backend::ristretto::RistrettoCtx, context::Ctx, serialization::StrandDeserialize};
use tracing::{event, instrument, Level};
use velvet::cli::state::State;
use velvet::cli::CliRun;
use velvet::fixtures::elections::Election;

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
pub async fn prepare_tally_for_area_contest(
    area_contest_plaintext: AreaContestDataType,
    base_tempdir: PathBuf,
    results_event_id_opt: &Option<String>,
    is_new: bool,
) -> Result<()> {
    let (plaintexts, tally_session_contest, contest, ballot_style) = area_contest_plaintext;

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

    //// create contest config file
    let contest_config_path: PathBuf = velvet_input_dir.join(format!(
        "default/configs/election__{election_id}/contest__{contest_id}/contest-config.json"
    ));
    let mut contest_config_file = fs::File::create(contest_config_path)?;

    writeln!(contest_config_file, "{}", serde_json::to_string(&contest)?)?;

    Ok(())
}

#[instrument(skip_all, err)]
pub async fn create_election_configs(
    area_contest_plaintexts: &Vec<AreaContestDataType>,
    base_tempdir: PathBuf,
) -> Result<()> {
    Ok(())
}

#[instrument(err)]
pub async fn run_velvet_tally(base_tally_path: PathBuf) -> Result<()> {
    //// Run Velvet
    let cli = CliRun {
        stage: "main".to_string(),
        pipe_id: "decode-ballots".to_string(),
        config: base_tally_path.join("velvet-config.json"),
        input_dir: base_tally_path.join("input-dir"),
        output_dir: base_tally_path.join("input-dir"),
    };

    let config = cli.validate()?;

    let mut state = State::new(&cli, &config)?;

    while let Some(next_stage) = state.get_next() {
        let stage_name = next_stage.to_string();
        event!(Level::INFO, "Exec {}", stage_name);
        state.exec_next()?;
    }
    Ok(())
}
