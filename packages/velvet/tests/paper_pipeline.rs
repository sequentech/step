// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A local paper-ballot workflow must carry the same independently counted
//! totals through tallying, winner selection and the reports shown to operators.

use serde_json::json;
use std::fs;
use velvet::cli::{state::State, test_all::generate_ballots, CliRun};
use velvet::fixtures::TestFixture;
use velvet::pipes::do_tally::{ContestResult, OUTPUT_CONTEST_RESULT_FILE};
use velvet::pipes::generate_reports::ReportData;
use velvet::pipes::pipe_inputs::{PipeInputs, CONTEST_CONFIG_FILE};
use velvet::pipes::pipe_name::PipeNameOutputDir;

struct PaperElection {
    // Keep the temporary files alive until all assertions have finished.
    _fixture: TestFixture,
    cli: CliRun,
    state: State,
    inputs: PipeInputs,
}

impl PaperElection {
    fn new(area_count: u32) -> Self {
        let fixture = TestFixture::new().unwrap();
        generate_ballots(&fixture, 1, 1, area_count, 0).unwrap();
        let cli = CliRun {
            stage: "main".into(),
            pipe_id: "do-tally".into(),
            config: fixture.config_path.clone(),
            input_dir: fixture.root_dir.join("tests/input-dir"),
            output_dir: fixture.root_dir.join("tests/output-dir"),
        };
        let mut config = cli.validate().unwrap();
        let report_config = velvet::config::generate_reports::PipeConfigGenerateReports {
            system_template: "{{{rendered_user_template}}}".into(),
            report_content_template: Some("{{#each reports}}{{#each candidate_result}}<p>{{candidate.id}}:{{total_count}}</p>{{/each}}{{/each}}".into()),
            ..Default::default()
        };
        let report_pipe = config
            .stages
            .stages_def
            .get_mut("main")
            .unwrap()
            .pipeline
            .iter_mut()
            .find(|pipe| pipe.id == "gen-report")
            .unwrap();
        report_pipe.config = Some(serde_json::to_value(report_config).unwrap());
        let state = State::new(&cli, &config).unwrap();
        let inputs = PipeInputs::new(cli.clone(), state.stages[0].clone()).unwrap();
        let contest = &inputs.election_list[0].contest_list[0];
        let mut contest_config = contest.contest.clone();
        contest_config.winning_candidates_num = 1;
        fs::write(
            contest.path.join(CONTEST_CONFIG_FILE),
            serde_json::to_vec(&contest_config).unwrap(),
        )
        .unwrap();
        Self {
            _fixture: fixture,
            cli,
            state,
            inputs,
        }
    }

    fn write_sheet(&self, area_index: usize, id: &str, marks: u64, blanks: u64) {
        let election = &self.inputs.election_list[0];
        let contest = &election.contest_list[0];
        let area = &contest.area_list[area_index];
        let candidate_id = &contest.contest.candidates[0].id;
        let sheets = PipeInputs::build_path(
            &self.inputs.root_path_tally_sheets,
            &election.id,
            Some(&contest.id),
            Some(&area.id),
        );
        let folder = PipeInputs::build_tally_sheet_path(&sheets, id);
        fs::create_dir_all(&folder).unwrap();
        let sheet = json!({
            "id": id, "tenant_id": contest.contest.tenant_id,
            "election_event_id": contest.contest.election_event_id,
            "election_id": election.id, "contest_id": contest.id, "area_id": area.id,
            "created_by_user_id": "test-operator", "status": "APPROVED", "version": 1,
            "channel": "PAPER",
            "content": {
                "area_id": area.id, "contest_id": contest.id, "census": 100,
                "total_valid_votes": marks + blanks, "total_blank_votes": blanks,
                "blank_ballots": blanks,
                "candidate_results": { candidate_id: {"candidate_id": candidate_id, "total_votes": marks} }
            }
        });
        fs::write(
            folder.join("tally-sheet.json"),
            serde_json::to_vec(&sheet).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn two_paper_batches_keep_their_breakdowns_and_combine_into_one_election_result() {
    // No electronic votes: the only input marks are these two approved sheets.
    let mut paper = PaperElection::new(1);
    paper.write_sheet(0, "batch-one", 3, 1);
    paper.write_sheet(0, "batch-two", 2, 0);
    let state = &mut paper.state;
    let cli = &paper.cli;
    let election = &paper.inputs.election_list[0];
    let contest = &election.contest_list[0];
    let area = &contest.area_list[0];
    let candidate_id = &contest.contest.candidates[0].id;

    state.exec_next().unwrap(); // Count the two paper batches.
    let area_output = PipeInputs::build_path(
        &cli.output_dir.join(PipeNameOutputDir::DoTally.as_ref()),
        &election.id,
        Some(&contest.id),
        Some(&area.id),
    );
    let total: ContestResult =
        serde_json::from_slice(&fs::read(area_output.join(OUTPUT_CONTEST_RESULT_FILE)).unwrap())
            .unwrap();
    assert_eq!(total.census, 100, "each sheet describes the same census");
    assert_eq!(total.total_votes, 6);
    assert_eq!(total.total_blank_votes, 1);
    assert_eq!(
        total
            .candidate_result
            .iter()
            .find(|result| result.candidate.id == *candidate_id)
            .unwrap()
            .total_count,
        5,
    );
    for (id, votes) in [("batch-one", 4), ("batch-two", 2)] {
        let breakdown: ContestResult = serde_json::from_slice(
            &fs::read(
                PipeInputs::build_tally_sheet_path(&area_output, id)
                    .join(OUTPUT_CONTEST_RESULT_FILE),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(breakdown.total_votes, votes);
    }

    state.exec_next().unwrap(); // Select the winner from the combined counts.
    state.exec_next().unwrap(); // Generate local HTML and JSON reports.
    let elections = state.get_results(true).unwrap();
    let report = elections
        .iter()
        .flat_map(|election| &election.reports)
        .find(|report| report.area.as_ref().is_none() && report.contest.is_some())
        .unwrap();
    assert_eq!(report.contest_result.as_ref().unwrap().total_votes, 6);
    let raw: ReportData = report.clone().into();
    assert_eq!(raw.election_id, election.id.to_string());
    assert_eq!(raw.tenant_id, contest.contest.tenant_id);
    assert_eq!(raw.winners.len(), 1);
    assert_eq!(raw.winners[0].candidate.id, *candidate_id);
    assert_eq!(raw.winners[0].total_count, 5);
}

#[test]
fn parent_area_reports_include_each_child_sheet_once_and_publish_a_consolidated_report() {
    use sequent_core::ballot::{ConsolidatedReportPolicy, ElectionPresentation};
    use velvet::pipes::pipe_inputs::{
        AreaConfig, ElectionConfig, AREA_CONFIG_FILE, ELECTION_CONFIG_FILE,
    };
    use walkdir::WalkDir;

    let mut paper = PaperElection::new(3);
    let election = &paper.inputs.election_list[0];
    let contest = &election.contest_list[0];
    let parent = &contest.area_list[0];
    let election_file = election.path.join(ELECTION_CONFIG_FILE);
    let mut config: ElectionConfig =
        serde_json::from_slice(&fs::read(&election_file).unwrap()).unwrap();
    config.presentation = Some(ElectionPresentation {
        consolidated_report_policy: Some(ConsolidatedReportPolicy::GENERATE),
        ..Default::default()
    });
    // Make one reporting parent and two leaf precincts. Parent records and
    // child records coexist; summing them blindly would double the ballots.
    for area in &contest.area_list[1..] {
        let area_file = area.path.join(AREA_CONFIG_FILE);
        let mut area_config: AreaConfig =
            serde_json::from_slice(&fs::read(&area_file).unwrap()).unwrap();
        area_config.parent_id = Some(parent.id);
        fs::write(area_file, serde_json::to_vec(&area_config).unwrap()).unwrap();
        config
            .areas
            .iter_mut()
            .find(|node| node.id == area.id.to_string())
            .unwrap()
            .parent_id = Some(parent.id.to_string());
    }
    fs::write(election_file, serde_json::to_vec(&config).unwrap()).unwrap();
    paper.write_sheet(1, "north", 3, 1);
    paper.write_sheet(2, "south", 2, 0);

    paper.state.exec_next().unwrap();
    paper.state.exec_next().unwrap();
    paper.state.exec_next().unwrap();
    let reports = paper.state.get_results(true).unwrap();
    let election_total = reports
        .iter()
        .flat_map(|election| &election.reports)
        .find(|report| report.area.is_none() && report.contest.is_some())
        .unwrap();
    assert_eq!(
        election_total.contest_result.as_ref().unwrap().total_votes,
        6
    );
    assert_eq!(
        election_total
            .contest_result
            .as_ref()
            .unwrap()
            .total_blank_votes,
        1
    );

    let consolidated = PipeInputs::build_consolidated_report_path(
        &paper
            .cli
            .output_dir
            .join(PipeNameOutputDir::GenerateReports.as_ref()),
        &election.id,
    );
    let html_files: Vec<_> = WalkDir::new(&consolidated)
        .into_iter()
        .map(|entry| entry.unwrap())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "html"))
        .collect();
    assert_eq!(html_files.len(), 1, "one consolidated report per election");
    let html = fs::read_to_string(html_files[0].path()).unwrap();
    assert!(html.contains(&format!("{}:5", contest.contest.candidates[0].id)));
}

#[test]
fn enabling_pdf_reports_publishes_a_pdf_beside_each_html_report() {
    use walkdir::WalkDir;

    let mut paper = PaperElection::new(1);
    paper.write_sheet(0, "paper", 2, 0);
    let report_pipe = paper.state.stages[0]
        .pipeline
        .iter_mut()
        .find(|pipe| pipe.id == "gen-report")
        .unwrap();
    report_pipe.config.as_mut().unwrap()["enable_pdfs"] = json!(true);
    paper.state.exec_next().unwrap();
    paper.state.exec_next().unwrap();
    paper.state.exec_next().unwrap();

    let reports = paper
        .cli
        .output_dir
        .join(PipeNameOutputDir::GenerateReports.as_ref());
    let html_files: Vec<_> = WalkDir::new(&reports)
        .into_iter()
        .map(|entry| entry.unwrap())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "html")
        })
        .collect();
    assert!(!html_files.is_empty());
    for html in html_files {
        // Each report's HTML and PDF are two views of the same result. This
        // catches partial publication and exercises the real local browser.
        let pdf = html.path().with_extension("pdf");
        let bytes = fs::read(&pdf).unwrap_or_else(|error| panic!("{}: {error}", pdf.display()));
        assert!(
            bytes.starts_with(b"%PDF-"),
            "{} is not a PDF",
            pdf.display()
        );
        assert!(bytes.len() > 100, "PDF must contain more than its header");
    }
}

#[test]
fn the_command_line_completes_a_paper_tally_and_returns_failure_for_missing_configuration() {
    use std::process::Command;

    let paper = PaperElection::new(1);
    paper.write_sheet(0, "paper", 2, 0);
    let run = Command::new(env!("CARGO_BIN_EXE_velvet"))
        .args(["run", "main", "do-tally", "--config"])
        .arg(&paper.cli.config)
        .arg("--input-dir")
        .arg(&paper.cli.input_dir)
        .arg("--output-dir")
        .arg(&paper.cli.output_dir)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let reports = paper.state.get_results(true).unwrap();
    let total = reports
        .iter()
        .flat_map(|election| &election.reports)
        .find(|report| report.area.is_none() && report.contest.is_some())
        .unwrap();
    assert_eq!(total.contest_result.as_ref().unwrap().total_votes, 2);

    let missing = Command::new(env!("CARGO_BIN_EXE_velvet"))
        .args(["run", "main", "do-tally", "--config"])
        .arg(paper.cli.config.with_extension("missing"))
        .arg("--input-dir")
        .arg(&paper.cli.input_dir)
        .arg("--output-dir")
        .arg(&paper.cli.output_dir)
        .output()
        .unwrap();
    assert!(
        !missing.status.success(),
        "invalid input must reach the process exit status"
    );
}
