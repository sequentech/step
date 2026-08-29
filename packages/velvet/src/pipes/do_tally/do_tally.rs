// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::tally;
use crate::pipes::{
    decode_ballots::OUTPUT_DECODED_BALLOTS_FILE,
    do_tally::counting_algorithm::utils::{
        get_area_tally_operation, get_area_weight, get_contest_tally_operation,
    },
    error::{Error, Result},
    pipe_inputs::{PipeInputs, PREFIX_TALLY_SHEET},
    pipe_name::PipeNameOutputDir,
    Pipe,
};

use crate::utils::HasId;
use rayon::prelude::*;
use sequent_core::{
    ballot::{BallotStyle, Candidate},
    ballot_style,
    services::area_tree::TreeNodeArea,
    sqlite::election_event,
    types::ceremonies::{ScopeOperation, TallyOperation},
    types::hasura::core::TallySheet,
    types::participation::VotesByChannel,
    types::tally_sheets::VotingChannel,
    util::path::{get_folder_name, list_subfolders},
};
use sequent_core::{
    ballot::{Contest, Weight},
    services::area_tree::TreeNode,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp;
use std::sync::Arc;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use tracing::{event, info, instrument, Level, Value as TracingValue};
use uuid::Uuid;

pub const OUTPUT_CONTEST_RESULT_FILE: &str = "contest_result.json";
pub const OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER: &str = "aggregate";
pub const INPUT_TALLY_SHEET_FILE: &str = "tally-sheet.json";
pub const OUTPUT_BREAKDOWNS_FOLDER: &str = "breakdowns";

pub struct DoTally {
    pub pipe_inputs: PipeInputs,
}

impl DoTally {
    #[instrument(skip_all, name = "DoTally::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        Self { pipe_inputs }
    }
}

#[instrument]
pub fn list_tally_sheet_subfolders(path: &Path) -> Vec<PathBuf> {
    let subfolders = list_subfolders(&path);
    let tally_sheet_folders: Vec<PathBuf> = subfolders
        .into_iter()
        .filter(|path| {
            let Some(folder_name) = get_folder_name(path) else {
                return false;
            };
            folder_name.starts_with(PREFIX_TALLY_SHEET)
        })
        .collect();
    tally_sheet_folders
}

fn load_tally_sheet_results(
    tally_sheets_dir: &Path,
    contest: &Contest,
) -> Result<Vec<(ContestResult, TallySheet)>> {
    if !tally_sheets_dir.is_dir() {
        return Ok(vec![]);
    }

    let tally_sheet_folders = list_tally_sheet_subfolders(tally_sheets_dir);
    if contest.is_acclaimed() && !tally_sheet_folders.is_empty() {
        return Err(Error::UnexpectedError(format!(
            "Acclaimed contest {} cannot have tally sheets",
            contest.id
        )));
    }

    tally_sheet_folders
        .into_iter()
        .map(|tally_sheet_folder| {
            let tally_sheet_file_path = tally_sheet_folder.join(INPUT_TALLY_SHEET_FILE);
            let tally_sheet_str = fs::read_to_string(&tally_sheet_file_path)
                .map_err(|error| Error::FileAccess(tally_sheet_file_path, error))?;
            let tally_sheet: TallySheet = serde_json::from_str(&tally_sheet_str)?;
            let contest_result = tally::process_tally_sheet(&tally_sheet, contest)
                .map_err(|error| Error::UnexpectedError(error.to_string()))?;
            validate_votes_by_channel(&contest_result)?;
            Ok((contest_result, tally_sheet))
        })
        .collect()
}

impl DoTally {
    #[instrument(err, skip_all)]
    fn save_tally_sheets_breakdown(
        &self,
        tally_sheet_results: &Vec<(ContestResult, TallySheet)>,
        base_file_path: &PathBuf,
    ) -> Result<()> {
        let base_breakdown_path = base_file_path.join(OUTPUT_BREAKDOWNS_FOLDER);
        let mut breakdown_map: HashMap<VotingChannel, ContestResult> = HashMap::new();

        for (contest_result, tally_sheet) in tally_sheet_results {
            let channel: VotingChannel = tally_sheet.channel.clone().into();

            breakdown_map
                .entry(channel)
                .and_modify(|current_result| {
                    *current_result = current_result.aggregate(contest_result, true);
                })
                .or_insert_with(|| contest_result.clone());
        }

        for (channel, contest_result) in breakdown_map {
            let breakdown_folder_path = base_breakdown_path.join(&channel.to_string());
            fs::create_dir_all(&breakdown_folder_path)?;
            let breakdown_file_path = breakdown_folder_path.join(OUTPUT_CONTEST_RESULT_FILE);
            let contest_result_file = fs::File::create(&breakdown_file_path)?;
            serde_json::to_writer(contest_result_file, &contest_result)?;
        }

        Ok(())
    }
}

fn participation_total(result: &ContestResult) -> Result<u64> {
    let declined = result
        .extended_metrics
        .as_ref()
        .map(|metrics| metrics.total_declined_to_vote)
        .unwrap_or_default();

    result
        .total_votes
        .checked_add(result.auditable_votes)
        .and_then(|total| total.checked_add(declined))
        .ok_or_else(|| Error::UnexpectedError("Participation total overflow".to_string()))
}

fn merge_votes_by_channel(aggregate: &mut VotesByChannel, counts: &VotesByChannel) -> Result<()> {
    for (channel, count) in counts {
        let current = aggregate.entry(channel.clone()).or_default();
        *current = current.checked_add(*count).ok_or_else(|| {
            Error::UnexpectedError(format!("Voting channel count overflow for {channel}"))
        })?;
    }
    Ok(())
}

fn merge_result_votes_by_channel(
    aggregate: &mut VotesByChannel,
    result: &ContestResult,
) -> Result<()> {
    if let Some(metrics) = &result.extended_metrics {
        merge_votes_by_channel(aggregate, &metrics.votes_by_channel)?;
    }
    Ok(())
}

fn aggregate_area_votes_by_channel<'a>(
    area_ids: impl IntoIterator<Item = &'a str>,
    votes_by_channel_map: &HashMap<String, Option<VotesByChannel>>,
) -> Result<(VotesByChannel, bool)> {
    let mut aggregate = VotesByChannel::new();
    let mut all_inputs_present = true;

    for area_id in area_ids {
        // The area tree contains every area in the election, including areas
        // whose ballot style does not contain this contest. Those areas are
        // not inputs for this aggregate and must not make it look incomplete.
        let Some(counts) = votes_by_channel_map.get(area_id) else {
            continue;
        };

        match counts {
            Some(counts) => merge_votes_by_channel(&mut aggregate, counts)?,
            None => all_inputs_present = false,
        }
    }

    Ok((aggregate, all_inputs_present))
}

fn set_votes_by_channel(result: &mut ContestResult, counts: VotesByChannel) {
    // Participation belongs to votes. An acclaimed result is canonical and
    // must not inherit the surrounding election's channel counts.
    if result.contest.is_acclaimed() {
        return;
    }
    result
        .extended_metrics
        .get_or_insert_with(ExtendedMetricsContest::default)
        .votes_by_channel = counts;
}

fn validate_votes_by_channel(result: &ContestResult) -> Result<()> {
    let has_counts = result
        .extended_metrics
        .as_ref()
        .is_some_and(|metrics| !metrics.votes_by_channel.is_empty());
    if !has_counts {
        return Ok(());
    }

    validate_complete_votes_by_channel(result)
}

fn validate_complete_votes_by_channel(result: &ContestResult) -> Result<()> {
    let channel_total = result
        .extended_metrics
        .as_ref()
        .map(|metrics| metrics.votes_by_channel.values())
        .into_iter()
        .flatten()
        .try_fold(0u64, |total, count| {
            total
                .checked_add(*count)
                .ok_or_else(|| Error::UnexpectedError("Voting channel total overflow".to_string()))
        })?;
    let participation_total = participation_total(result)?;

    if channel_total != participation_total {
        return Err(Error::UnexpectedError(format!(
            "Voting channel total {channel_total} does not match participation total {participation_total} for contest {}",
            result.contest.id
        )));
    }

    Ok(())
}

fn has_complete_votes_by_channel(result: &ContestResult) -> Result<bool> {
    validate_votes_by_channel(result)?;
    let has_counts = result
        .extended_metrics
        .as_ref()
        .is_some_and(|metrics| !metrics.votes_by_channel.is_empty());
    Ok(participation_total(result)? == 0 || has_counts)
}

impl Pipe for DoTally {
    #[instrument(err, skip_all, name = "DoTally::exec")]
    fn exec(&self) -> Result<()> {
        let input_dir_base = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DecodeBallots.as_ref());
        let output_dir_base = self
            .pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::DoTally.as_ref());
        let tally_sheets_dir_base = self.pipe_inputs.root_path_tally_sheets.clone();

        for election_input in &self.pipe_inputs.election_list {
            // Parallelize the processing of each contest
            election_input
                .contest_list
                .par_iter()
                .map(|contest_input| {
                    let input_dir = input_dir_base.clone();
                    let output_dir = output_dir_base.clone();
                    let tally_sheets_dir = tally_sheets_dir_base.clone();

                    // These are specific to the contest and need to be cloned for use in area processing.
                    let election_id_for_contest = contest_input.election_id.clone();
                    let contest_id_for_contest = contest_input.id.clone();
                    let contest_object = contest_input.contest.clone();
                    let contest_op = get_contest_tally_operation(&contest_object);

                    // --- Start of logic for a single contest ---
                    let _areas_info: Vec<TreeNodeArea> = contest_input // Renamed, original `areas` was unused after info
                        .area_list
                        .iter()
                        .map(|area| (&area.area).into())
                        .collect();
                    info!(
                        "areas for contest {}: {:?}",
                        contest_id_for_contest, _areas_info
                    );

                    let areas_tree = Arc::new(
                        TreeNode::<()>::from_areas(election_input.areas.clone()).map_err(
                            |err| {
                                Error::UnexpectedError(format!(
                                    "Error building area tree for contest {}: {:?}",
                                    contest_id_for_contest, err
                                ))
                            },
                        )?,
                    );

                    let census_map: HashMap<String, u64> = contest_input
                        .area_list
                        .iter()
                        .map(|area_input| (area_input.area.id.to_string(), area_input.census))
                        .collect();
                    let auditable_votes_map: HashMap<String, u64> = contest_input
                        .area_list
                        .iter()
                        .map(|area_input| {
                            (area_input.area.id.to_string(), area_input.auditable_votes)
                        })
                        .collect();
                    let votes_by_channel_map: HashMap<String, Option<VotesByChannel>> =
                        contest_input
                            .area_list
                            .iter()
                            .map(|area_input| {
                                (
                                    area_input.area.id.to_string(),
                                    area_input.area.votes_by_channel.clone(),
                                )
                            })
                            .collect();

                    // Parallelize processing for each area within this contest
                    let area_processing_results: Result<Vec<_>, Error> = contest_input
                        .area_list
                        .par_iter()
                        .map(|area_input| {
                            // Clone data needed per area task.
                            let area_id = area_input.id.clone();
                            let election_id = election_id_for_contest.clone();
                            let contest_id = contest_id_for_contest.clone();

                            let base_input_path = PipeInputs::build_path(
                                &input_dir,
                                &election_id,
                                Some(&contest_id),
                                Some(&area_id),
                            );

                            let base_output_path = PipeInputs::build_path(
                                &output_dir,
                                &election_id,
                                Some(&contest_id),
                                Some(&area_id),
                            );

                            let decoded_ballots_file =
                                base_input_path.join(OUTPUT_DECODED_BALLOTS_FILE);

                            // Create aggregate tally from children areas
                            let Some(area_tree_node) =
                                areas_tree.as_ref().find_area(&area_input.id.to_string())
                            else {
                                return Err(Error::UnexpectedError(format!(
                                    "Error finding area {} in areas tree for contest {}",
                                    area_input.id, contest_id
                                )));
                            };
                            let children_areas = area_tree_node.get_all_children();
                            let num_children_areas = children_areas
                                .iter()
                                .filter(|child| child.id != area_input.id.to_string())
                                .count();

                            let area_op = get_area_tally_operation(
                                &election_input.ballot_styles,
                                contest_object.get_counting_algorithm(),
                                &area_input.id,
                            );

                            if num_children_areas > 0usize {
                                let base_aggregate_path = base_output_path
                                    .join(OUTPUT_CONTEST_RESULT_AREA_CHILDREN_AGGREGATE_FOLDER);
                                fs::create_dir_all(&base_aggregate_path)?;

                                let census_size: u64 = children_areas
                                    .iter()
                                    .filter_map(|child_area| {
                                        census_map.get(&child_area.id).copied()
                                    })
                                    .sum();
                                let auditable_votes_size: u64 = children_areas
                                    .iter()
                                    .filter_map(|child_area| {
                                        auditable_votes_map.get(&child_area.id).copied()
                                    })
                                    .sum();

                                let children_area_paths: Vec<(PathBuf, Weight)> = children_areas
                                    .iter()
                                    .map(|child_area| -> Result<(PathBuf, Weight), Error> {
                                        let child_area_id = Uuid::parse_str(&child_area.id)
                                            .map_err(|err| {
                                                Error::UnexpectedError(format!(
                                                    "Uuid parse error: {err:?}"
                                                ))
                                            })?;

                                        let child_area_weight = get_area_weight(
                                            &election_input.ballot_styles,
                                            &child_area_id,
                                        );

                                        Ok((
                                            PipeInputs::build_path(
                                                &input_dir,
                                                &election_id,
                                                Some(&contest_id),
                                                Some(&child_area_id),
                                            )
                                            .join(OUTPUT_DECODED_BALLOTS_FILE),
                                            child_area_weight,
                                        ))
                                    })
                                    .collect::<Result<Vec<(PathBuf, Weight)>, Error>>()?;

                                let counting_algorithm = tally::create_tally(
                                    &contest_object,
                                    ScopeOperation::Area(area_op), // The operation of the parent area is used in the aggregate of its children, this makes sense so that each child has the same data available
                                    children_area_paths,
                                    census_size,
                                    auditable_votes_size,
                                    vec![],
                                    vec![],
                                )
                                .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                                let mut aggregate_result: ContestResult = counting_algorithm
                                    .tally(&mut rand::rng())
                                    .map_err(|e| Error::UnexpectedError(e.to_string()))?;

                                let (electronic_channel_counts, all_channel_inputs_present) =
                                    aggregate_area_votes_by_channel(
                                        children_areas.iter().map(|area| area.id.as_str()),
                                        &votes_by_channel_map,
                                    )?;
                                set_votes_by_channel(
                                    &mut aggregate_result,
                                    electronic_channel_counts,
                                );
                                let has_complete_electronic_channels = if all_channel_inputs_present
                                {
                                    validate_complete_votes_by_channel(&aggregate_result)?;
                                    true
                                } else {
                                    participation_total(&aggregate_result)? == 0
                                };

                                let mut aggregate_tally_sheet_results = vec![];
                                for child_area in &children_areas {
                                    let child_area_id =
                                        Uuid::parse_str(&child_area.id).map_err(|error| {
                                            Error::UnexpectedError(format!(
                                                "Uuid parse error: {error:?}"
                                            ))
                                        })?;
                                    let child_tally_sheets_dir = PipeInputs::build_path(
                                        &tally_sheets_dir,
                                        &election_id,
                                        Some(&contest_id),
                                        Some(&child_area_id),
                                    );
                                    aggregate_tally_sheet_results.extend(load_tally_sheet_results(
                                        &child_tally_sheets_dir,
                                        &contest_object,
                                    )?);
                                }

                                aggregate_result = aggregate_tally_sheet_results.iter().fold(
                                    aggregate_result,
                                    |result, (tally_sheet_result, _)| {
                                        result.aggregate(tally_sheet_result, false)
                                    },
                                );
                                if !has_complete_electronic_channels {
                                    set_votes_by_channel(
                                        &mut aggregate_result,
                                        VotesByChannel::new(),
                                    );
                                }
                                validate_votes_by_channel(&aggregate_result)?;

                                let file_path =
                                    base_aggregate_path.join(OUTPUT_CONTEST_RESULT_FILE);
                                let file = fs::File::create(file_path)?;
                                serde_json::to_writer_pretty(file, &aggregate_result)?;
                                // Using pretty for readability
                            }

                            let area_weight =
                                get_area_weight(&election_input.ballot_styles, &area_input.id);

                            // Create area tally
                            let counting_algorithm_area = tally::create_tally(
                                &contest_object,
                                ScopeOperation::Area(area_op),
                                vec![(decoded_ballots_file.clone(), area_weight)],
                                area_input.census,
                                area_input.auditable_votes,
                                vec![],
                                vec![],
                            )
                            .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                            let mut area_tally_results = counting_algorithm_area
                                .tally(&mut rand::rng())
                                .map_err(|e| Error::UnexpectedError(e.to_string()))?;

                            let has_channel_input = area_input.area.votes_by_channel.is_some();
                            if !area_tally_results.contest.is_acclaimed() {
                                let extended_metrics = area_tally_results
                                    .extended_metrics
                                    .get_or_insert_with(ExtendedMetricsContest::default);
                                extended_metrics.weight = area_weight;
                            }
                            set_votes_by_channel(
                                &mut area_tally_results,
                                area_input.area.votes_by_channel.clone().unwrap_or_default(),
                            );
                            let has_complete_electronic_channels = if has_channel_input {
                                validate_complete_votes_by_channel(&area_tally_results)?;
                                true
                            } else {
                                participation_total(&area_tally_results)? == 0
                            };

                            // Tally sheets tally for this area
                            let input_tally_sheets_dir_path = PipeInputs::build_path(
                                &tally_sheets_dir,
                                &election_id,
                                Some(&contest_id),
                                Some(&area_id),
                            );
                            let area_specific_tally_sheet_results = load_tally_sheet_results(
                                &input_tally_sheets_dir_path,
                                &contest_object,
                            )?;
                            for (contest_result_sheet, tally_sheet) in
                                &area_specific_tally_sheet_results
                            {
                                let output_tally_sheets_folder_path =
                                    PipeInputs::build_tally_sheet_path(
                                        &base_output_path,
                                        &tally_sheet.id,
                                    );
                                fs::create_dir_all(&output_tally_sheets_folder_path)?;
                                let output_tally_sheets_file_path = output_tally_sheets_folder_path
                                    .join(OUTPUT_CONTEST_RESULT_FILE);
                                let contest_result_file_sheet =
                                    fs::File::create(&output_tally_sheets_file_path)?;
                                serde_json::to_writer_pretty(
                                    contest_result_file_sheet,
                                    contest_result_sheet,
                                )?;
                            }

                            let mut area_result_with_tally_sheets =
                                area_specific_tally_sheet_results.iter().fold(
                                    area_tally_results.clone(),
                                    |result, (tally_sheet_result, _)| {
                                        result.aggregate(tally_sheet_result, false)
                                    },
                                );
                            if !has_complete_electronic_channels {
                                set_votes_by_channel(
                                    &mut area_result_with_tally_sheets,
                                    VotesByChannel::new(),
                                );
                            }
                            validate_votes_by_channel(&area_result_with_tally_sheets)?;

                            fs::create_dir_all(&base_output_path)?;
                            let file_path_area = base_output_path.join(OUTPUT_CONTEST_RESULT_FILE);
                            let file_area = fs::File::create(file_path_area)?;
                            serde_json::to_writer_pretty(
                                file_area,
                                &area_result_with_tally_sheets,
                            )?;

                            // Return data needed for final aggregation for the contest
                            Ok((
                                (decoded_ballots_file, area_weight),
                                area_input.census,
                                area_input.auditable_votes,
                                area_specific_tally_sheet_results,
                                area_tally_results,
                            ))
                        })
                        .collect(); // Collects Result<Vec<(PathBuf, u64, u64, Vec<_>)>, Error>

                    let collected_area_outputs = area_processing_results?; // Propagate error if any area failed

                    // Aggregate results from parallel area processing
                    let mut contest_ballot_files: Vec<(PathBuf, Weight)> = vec![];
                    let mut sum_census: u64 = 0;
                    let mut sum_auditable_votes: u64 = 0;
                    let mut tally_sheet_results_for_contest: Vec<(ContestResult, TallySheet)> =
                        vec![];
                    let mut area_tally_results_for_contest: Vec<ContestResult> = vec![];

                    for (
                        ballot_file,
                        census,
                        auditable_votes_val,
                        sheet_results,
                        area_tally_results,
                    ) in collected_area_outputs
                    {
                        contest_ballot_files.push(ballot_file);
                        sum_census += census;
                        sum_auditable_votes += auditable_votes_val;
                        tally_sheet_results_for_contest.extend(sheet_results);
                        area_tally_results_for_contest.push(area_tally_results);
                    }

                    // Create contest-level output path (directory for the contest)
                    let contest_output_dir_path = PipeInputs::build_path(
                        &output_dir, // This is the output_dir cloned for this contest task
                        &election_id_for_contest,
                        Some(&contest_id_for_contest),
                        None, // No area_id for contest-level summary
                    );
                    fs::create_dir_all(&contest_output_dir_path)?; // Ensure contest directory exists

                    self.save_tally_sheets_breakdown(
                        &tally_sheet_results_for_contest,
                        &contest_output_dir_path,
                    )?;

                    let final_only_sheet_results: Vec<ContestResult> =
                        tally_sheet_results_for_contest
                            .iter()
                            .map(|(res, _)| res.clone())
                            .collect();
                    let has_complete_electronic_channels = area_tally_results_for_contest
                        .iter()
                        .try_fold(true, |is_complete, area_result| {
                            Ok::<bool, Error>(
                                is_complete && has_complete_votes_by_channel(area_result)?,
                            )
                        })?;
                    let mut final_channel_counts = VotesByChannel::new();
                    if has_complete_electronic_channels {
                        for result in area_tally_results_for_contest
                            .iter()
                            .chain(final_only_sheet_results.iter())
                        {
                            merge_result_votes_by_channel(&mut final_channel_counts, result)?;
                        }
                    }

                    // Create final contest tally
                    let final_counting_algorithm = tally::create_tally(
                        &contest_object,
                        ScopeOperation::Contest(contest_op),
                        contest_ballot_files,
                        sum_census,
                        sum_auditable_votes,
                        final_only_sheet_results,
                        area_tally_results_for_contest,
                    )
                    .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                    let mut final_res = final_counting_algorithm
                        .tally(&mut rand::rng())
                        .map_err(|e| Error::UnexpectedError(e.to_string()))?;
                    set_votes_by_channel(&mut final_res, final_channel_counts);
                    validate_votes_by_channel(&final_res)?;

                    let final_contest_result_file_path =
                        contest_output_dir_path.join(OUTPUT_CONTEST_RESULT_FILE);
                    let final_file = fs::File::create(final_contest_result_file_path)?;
                    serde_json::to_writer_pretty(final_file, &final_res)?; // Using pretty

                    Ok(()) // Result for this contest's processing
                })
                .collect::<Result<Vec<()>, Error>>()?; // Collect results from parallel contest processing
        }
        Ok(())
    }
}

// Tally result types live in `velvet-core` so they can be shared with WASM
// consumers (the workbench). Re-exported here to preserve the existing
// `crate::pipes::do_tally::{ContestResult, CandidateResult, InvalidVotes,
// ExtendedMetricsContest}` import paths inside velvet. (`total_blank_ballots`
// forward-ported from main #2989; `ExtendedMetricsElection` removed there as
// dead — both now reflected in velvet-core's `result.rs`.)
pub use velvet_core::result::{
    CandidateResult, ContestResult, ExtendedMetricsContest, InvalidVotes,
};

// `HasId` is velvet-internal so the impls remain here.
impl HasId for Contest {
    fn id(&self) -> &str {
        &self.id
    }
}

impl HasId for Candidate {
    fn id(&self) -> &str {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::{
        ballot::VotingStatusChannel, types::tally_sheets::VotingChannel as TallySheetVotingChannel,
    };
    use tempfile::tempdir;

    #[test]
    fn rejects_tally_sheet_artifacts_for_an_acclaimed_contest() {
        let tally_sheets_dir = tempdir().expect("temporary tally sheet directory");
        fs::create_dir(tally_sheets_dir.path().join("tally_sheet__unexpected"))
            .expect("tally sheet folder");
        let contest = Contest {
            id: "acclaimed".to_string(),
            is_acclaimed: Some(true),
            ..Contest::default()
        };

        let error = load_tally_sheet_results(tally_sheets_dir.path(), &contest)
            .expect_err("acclaimed tally sheets must be rejected");

        assert!(error.to_string().contains("cannot have tally sheets"));
    }

    #[test]
    fn acclaimed_result_cannot_inherit_election_channel_counts() {
        let mut result = ContestResult {
            contest: Contest {
                is_acclaimed: Some(true),
                ..Contest::default()
            },
            extended_metrics: Some(ExtendedMetricsContest::default()),
            ..ContestResult::default()
        };

        set_votes_by_channel(
            &mut result,
            VotesByChannel::from([(VotingStatusChannel::ONLINE.into(), 12)]),
        );

        assert!(result
            .extended_metrics
            .expect("extended metrics")
            .votes_by_channel
            .is_empty());
    }

    #[test]
    fn extended_metrics_aggregate_channel_counts() {
        let left = ExtendedMetricsContest {
            votes_by_channel: VotesByChannel::from([
                (VotingStatusChannel::ONLINE.into(), 3),
                (VotingStatusChannel::TELEPHONE.into(), 1),
            ]),
            ..Default::default()
        };
        let right = ExtendedMetricsContest {
            votes_by_channel: VotesByChannel::from([
                (VotingStatusChannel::ONLINE.into(), 2),
                (TallySheetVotingChannel::PAPER.into(), 4),
            ]),
            ..Default::default()
        };

        let aggregate = left.aggregate(&right);

        assert_eq!(
            aggregate
                .votes_by_channel
                .get(&VotingStatusChannel::ONLINE.into()),
            Some(&5)
        );
        assert_eq!(
            aggregate
                .votes_by_channel
                .get(&VotingStatusChannel::TELEPHONE.into()),
            Some(&1)
        );
        assert_eq!(
            aggregate
                .votes_by_channel
                .get(&TallySheetVotingChannel::PAPER.into()),
            Some(&4)
        );
    }

    #[test]
    fn extended_metrics_aggregate_sums_blank_ballots_across_areas() {
        let left = ExtendedMetricsContest {
            total_blank_ballots: 2,
            ..Default::default()
        };
        let right = ExtendedMetricsContest {
            total_blank_ballots: 3,
            ..Default::default()
        };

        let aggregate = left.aggregate(&right);

        assert_eq!(aggregate.total_blank_ballots, 5);
    }

    #[test]
    fn contest_result_aggregation_preserves_auditable_channel_participation() {
        let area_result = ContestResult {
            census: 1,
            total_votes: 1,
            auditable_votes: 1,
            extended_metrics: Some(ExtendedMetricsContest {
                votes_by_channel: VotesByChannel::from([(VotingStatusChannel::ONLINE.into(), 2)]),
                ..Default::default()
            }),
            ..Default::default()
        };

        let aggregate = ContestResult::default().aggregate(&area_result, true);

        assert_eq!(aggregate.auditable_votes, 1);
        assert!(validate_votes_by_channel(&aggregate).is_ok());
    }

    #[test]
    fn area_channel_aggregation_ignores_areas_outside_the_contest() {
        let area_ids = ["parent", "contest-child", "other-contest-child"];
        let votes_by_channel_map = HashMap::from([
            (
                "parent".to_string(),
                Some(VotesByChannel::from([(
                    VotingStatusChannel::ONLINE.into(),
                    2,
                )])),
            ),
            (
                "contest-child".to_string(),
                Some(VotesByChannel::from([(
                    VotingStatusChannel::TELEPHONE.into(),
                    1,
                )])),
            ),
        ]);

        let (aggregate, all_inputs_present) =
            aggregate_area_votes_by_channel(area_ids.iter().copied(), &votes_by_channel_map)
                .unwrap();

        assert!(all_inputs_present);
        assert_eq!(aggregate.get(&VotingStatusChannel::ONLINE.into()), Some(&2));
        assert_eq!(
            aggregate.get(&VotingStatusChannel::TELEPHONE.into()),
            Some(&1)
        );
    }

    #[test]
    fn area_channel_aggregation_detects_a_legacy_contest_area() {
        let votes_by_channel_map = HashMap::from([
            (
                "parent".to_string(),
                Some(VotesByChannel::from([(
                    VotingStatusChannel::ONLINE.into(),
                    2,
                )])),
            ),
            ("legacy-child".to_string(), None),
        ]);

        let (_, all_inputs_present) =
            aggregate_area_votes_by_channel(["parent", "legacy-child"], &votes_by_channel_map)
                .unwrap();

        assert!(!all_inputs_present);
    }

    #[test]
    fn channel_validation_includes_auditable_and_declined_ballots() {
        let result = ContestResult {
            total_votes: 4,
            auditable_votes: 1,
            extended_metrics: Some(ExtendedMetricsContest {
                total_declined_to_vote: 1,
                votes_by_channel: VotesByChannel::from([(VotingStatusChannel::ONLINE.into(), 6)]),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert!(validate_votes_by_channel(&result).is_ok());
    }

    #[test]
    fn legacy_participation_without_channel_counts_is_incomplete_but_valid() {
        let result = ContestResult {
            total_votes: 2,
            extended_metrics: Some(ExtendedMetricsContest::default()),
            ..Default::default()
        };

        assert!(validate_votes_by_channel(&result).is_ok());
        assert!(!has_complete_votes_by_channel(&result).unwrap());
    }

    #[test]
    fn channel_validation_rejects_a_partial_non_empty_breakdown() {
        let result = ContestResult {
            total_votes: 2,
            extended_metrics: Some(ExtendedMetricsContest {
                votes_by_channel: VotesByChannel::from([(VotingStatusChannel::ONLINE.into(), 1)]),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert!(validate_votes_by_channel(&result).is_err());
    }
}
