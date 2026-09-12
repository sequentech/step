// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Report ordering follows the configured ballot presentation. Re-sorting a
//! randomized ballot while publishing results would lose the recorded order.
use super::*;
use sequent_core::ballot::CandidatePresentation;

fn candidate_result(id: &str, position: Option<i64>) -> CandidateResult {
    CandidateResult {
        candidate: Candidate {
            id: id.into(),
            name: Some(id.into()),
            presentation: position.map(|sort_order| CandidatePresentation {
                sort_order: Some(sort_order),
                ..Default::default()
            }),
            ..Default::default()
        },
        total_count: 7,
        percentage_votes: 50.0,
    }
}

#[test]
fn custom_candidate_order_honors_positions_and_keeps_missing_positions_first() {
    let mut candidates = vec![
        candidate_result("third", Some(3)),
        candidate_result("first", None),
        candidate_result("second", Some(2)),
    ];
    sort_candidates(&mut candidates, CandidatesOrder::Custom);
    assert_eq!(
        candidates
            .iter()
            .map(|entry| entry.candidate.id.as_str())
            .collect::<Vec<_>>(),
        ["first", "second", "third"]
    );
    assert!(
        candidates.iter().all(|entry| entry.total_count == 7),
        "sorting must preserve counts"
    );
}

#[test]
fn randomized_candidate_order_preserves_the_existing_snapshot() {
    let mut candidates = vec![
        candidate_result("Zulu", Some(1)),
        candidate_result("Alpha", Some(0)),
    ];
    sort_candidates(&mut candidates, CandidatesOrder::Random);
    assert_eq!(
        candidates
            .iter()
            .map(|entry| entry.candidate.id.as_str())
            .collect::<Vec<_>>(),
        ["Zulu", "Alpha"]
    );
}

fn report_pipe(config: PipeConfigGenerateReports) -> GenerateReports {
    use crate::cli::{state::Stage, CliRun};
    use crate::config::PipeConfig;
    use crate::pipes::pipe_name::PipeName;

    GenerateReports::new(PipeInputs {
        cli: CliRun {
            stage: "main".into(),
            pipe_id: "reports".into(),
            config: PathBuf::new(),
            input_dir: PathBuf::new(),
            output_dir: PathBuf::new(),
        },
        root_path_config: PathBuf::new(),
        root_path_ballots: PathBuf::new(),
        root_path_tally_sheets: PathBuf::new(),
        root_path_database: PathBuf::new(),
        stage: Stage {
            name: "main".into(),
            current_pipe: Some(PipeName::GenerateReports),
            previous_pipe: None,
            pipeline: vec![PipeConfig {
                id: "reports".into(),
                pipe: PipeName::GenerateReports,
                config: Some(serde_json::to_value(config).unwrap()),
            }],
        },
        election_list: vec![],
    })
}

fn local_template() -> PipeConfigGenerateReports {
    PipeConfigGenerateReports {
        system_template: "<main>{{{rendered_user_template}}}</main>".into(),
        report_content_template: Some("<p>Result summary</p>".into()),
        ..Default::default()
    }
}

#[test]
fn template_failures_are_reported_before_an_artifact_can_be_published() {
    let valid = report_pipe(local_template());
    let (bytes, _) = valid
        .generate_report(vec![], false, None, &HashMap::new(), false)
        .unwrap();
    assert!(String::from_utf8(bytes.bytes_html)
        .unwrap()
        .contains("Result summary"));
    assert!(bytes.bytes_pdf.is_none());

    let mut content_error = local_template();
    content_error.report_content_template = Some("{{#if}}".into());
    let mut system_error = local_template();
    system_error.system_template = "{{#if}}".into();
    for config in [content_error, system_error] {
        let outcome =
            report_pipe(config).generate_report(vec![], false, None, &HashMap::new(), false);
        assert!(
            matches!(outcome, Err(Error::UnexpectedError(message)) if message.contains("template"))
        );
    }
}

#[test]
fn pdf_renderer_failures_keep_the_browser_error_in_the_result() {
    use sequent_core::types::templates::PrintToPdfOptionsLocal;
    let mut config = local_template();
    // Chrome rejects a zero scale. This reaches a real browser-side failure
    // after successful HTML rendering without needing a broken executable.
    config.pdf_options = Some(PrintToPdfOptionsLocal {
        scale: Some(0.0),
        ..Default::default()
    });
    let outcome = report_pipe(config).generate_report(vec![], true, None, &HashMap::new(), false);
    assert!(
        matches!(outcome, Err(Error::UnexpectedError(message)) if message.contains("html_to_pdf") && message.to_lowercase().contains("scale"))
    );
}

#[test]
fn winner_order_override_puts_elected_candidates_first_then_uses_counts_and_names() {
    let mut contest = Contest::default();
    contest.annotations = Some(HashMap::from([(
        CONTEST_REPORT_CONFIG.into(),
        serde_json::json!({"candidates_order": "SortByWinningPosition"}).to_string(),
    )]));
    let candidates = vec![
        candidate_result("Beta", None),
        candidate_result("Zulu", None),
        candidate_result("Alpha", None),
    ];
    let result = ContestResult {
        contest: contest.clone(),
        candidate_result: candidates.clone(),
        ..Default::default()
    };
    let report = ReportData {
        election_name: "Council".into(),
        election_alias: "Council".into(),
        election_id: "election".into(),
        election_event_id: "event".into(),
        tenant_id: "tenant".into(),
        election_description: String::new(),
        election_dates: None,
        election_annotations: HashMap::new(),
        election_event_annotations: HashMap::new(),
        contest: Some(contest),
        area: None,
        contest_result: Some(result),
        winners: vec![WinnerResult {
            candidate: candidates[1].candidate.clone(),
            total_count: 7,
            winning_position: 1,
        }],
        channel_type: None,
        election_results: None,
    };
    let computed = report_pipe(local_template())
        .compute_reports(vec![report], &HashMap::new(), false)
        .unwrap();
    assert_eq!(
        computed[0]
            .candidate_result
            .iter()
            .map(|entry| entry.candidate.id.as_str())
            .collect::<Vec<_>>(),
        ["Zulu", "Alpha", "Beta"]
    );
    assert_eq!(computed[0].candidate_result[0].winning_position, Some(1));
    assert!(computed[0].candidate_result[1..]
        .iter()
        .all(|entry| entry.winning_position.is_none()));
}
