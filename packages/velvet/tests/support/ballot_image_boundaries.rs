// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Receipt content must preserve selected candidates and deterministic ordering.
//! These internal tests inspect private presentation helpers without exposing
//! them as public API or requiring a browser for every formatting edge case.

use super::*;
use crate::cli::{state::Stage, CliRun};
use crate::pipes::pipe_inputs::InputContestConfig;
use sequent_core::ballot::CandidatePresentation;
use sequent_core::ballot_codec::multi_ballot::{DecodedContestChoice, DecodedContestChoices};
use serde_json::json;
use tempfile::tempdir;
use uuid::Uuid;

const STAGE_ID: &str = "main";
const PIPE_ID: &str = "images";

fn candidate(id: &str, name: &str) -> Candidate {
    Candidate {
        id: id.into(),
        name: Some(name.into()),
        ..Default::default()
    }
}

fn contest() -> Contest {
    Contest {
        id: Uuid::from_u128(1).to_string(),
        name: Some("Council".into()),
        max_votes: 2,
        candidates: vec![candidate("ada", "Ada"), candidate("bea", "Bea")],
        ..Default::default()
    }
}

fn election() -> InputElectionConfig {
    InputElectionConfig {
        id: Uuid::from_u128(2),
        name: "Council election".into(),
        alias: "Council".into(),
        description: String::new(),
        dates: None,
        annotations: HashMap::new(),
        election_event_annotations: HashMap::new(),
        ballot_styles: vec![],
        contest_list: vec![InputContestConfig {
            id: Uuid::from_u128(1),
            election_id: Uuid::from_u128(2),
            contest: contest(),
            area_list: vec![],
            path: PathBuf::new(),
        }],
        path: PathBuf::new(),
        census: 10,
        total_votes: 2,
        areas: vec![],
        presentation: None,
    }
}

fn ballot(selected: &[&str]) -> DecodedBallotChoices {
    DecodedBallotChoices {
        is_explicit_invalid: false,
        is_blank_ballot: false,
        serial_number: Some("000000001".into()),
        choices: vec![DecodedContestChoices {
            contest_id: Uuid::from_u128(1).to_string(),
            is_explicit_invalid: false,
            choices: selected
                .iter()
                .map(|id| DecodedContestChoice((*id).into()))
                .collect(),
            invalid_errors: vec![],
            invalid_alerts: vec![],
        }],
    }
}

fn image_pipe() -> MCBallotImages {
    MCBallotImages::new(PipeInputs {
        cli: CliRun {
            stage: STAGE_ID.into(),
            pipe_id: PIPE_ID.into(),
            config: PathBuf::new(),
            input_dir: PathBuf::new(),
            output_dir: PathBuf::new(),
        },
        root_path_config: PathBuf::new(),
        root_path_ballots: PathBuf::new(),
        root_path_tally_sheets: PathBuf::new(),
        root_path_database: PathBuf::new(),
        stage: Stage {
            name: STAGE_ID.into(),
            pipeline: vec![],
            current_pipe: None,
            previous_pipe: None,
        },
        election_list: vec![],
    })
}

#[test]
fn receipt_conversion_keeps_unselected_candidates_and_rejects_unknown_identities() {
    let election = election();
    let bridges = convert_ballots(&election, vec![ballot(&["bea"])]).unwrap();
    let choices = &bridges[0].choices[0].choices;
    assert_eq!(
        choices
            .iter()
            .map(|c| (c.id.as_str(), c.selected))
            .collect::<Vec<_>>(),
        [("ada", -1), ("bea", 1)]
    );
    assert!(convert_ballots(&election, vec![ballot(&["unknown"])]).is_err());

    let mut wrong_contest = ballot(&["ada"]);
    wrong_contest.choices[0].contest_id = Uuid::from_u128(3).to_string();
    assert!(convert_ballots(&election, vec![wrong_contest]).is_err());
}

#[test]
fn candidate_order_uses_aliases_case_insensitively_and_custom_positions_when_requested() {
    let contest = contest();
    let bridges = convert_ballots(&election(), vec![ballot(&["ada"])]).unwrap();
    let mut choices = DecodedChoice::from_dvcs(&bridges[0].choices[0], &contest);
    choices[0].candidate.as_mut().unwrap().alias = Some("Zulu".into());
    // Uppercase Z sorts before lowercase a bytewise; the receipt contract is
    // case-insensitive and must put alpha first.
    choices[1].candidate.as_mut().unwrap().name = Some("alpha".into());
    sort_candidates(&mut choices, CandidatesOrder::Alphabetical);
    assert_eq!(
        choices
            .iter()
            .map(|c| c.choice.id.as_str())
            .collect::<Vec<_>>(),
        ["bea", "ada"]
    );

    choices[0].candidate.as_mut().unwrap().presentation = Some(CandidatePresentation {
        sort_order: Some(2),
        ..Default::default()
    });
    choices[1].candidate.as_mut().unwrap().presentation = Some(CandidatePresentation {
        sort_order: Some(1),
        ..Default::default()
    });
    sort_candidates(&mut choices, CandidatesOrder::Custom);
    assert_eq!(
        choices
            .iter()
            .map(|c| c.choice.id.as_str())
            .collect::<Vec<_>>(),
        ["ada", "bea"]
    );
    sort_candidates(&mut choices, CandidatesOrder::Random);
    assert_eq!(
        choices
            .iter()
            .map(|c| c.choice.id.as_str())
            .collect::<Vec<_>>(),
        ["ada", "bea"],
        "receipts must preserve their snapshot rather than randomizing again"
    );
}

#[test]
fn missing_candidate_metadata_has_a_deterministic_sort_position() {
    let bridges = convert_ballots(&election(), vec![ballot(&["ada"])]).unwrap();
    let mut choices = DecodedChoice::from_dvcs(&bridges[0].choices[0], &contest());
    choices[1].candidate = None;
    sort_candidates(&mut choices, CandidatesOrder::Alphabetical);
    assert_eq!(choices[0].choice.id, "bea");
    // Alphabetical sorting moved the missing metadata to index 0. Index 1 is
    // Ada's populated entry; the next check exercises a different ordering.
    assert!(choices[0].candidate.is_none());
    assert_eq!(choices[1].choice.id, "ada");
    choices[1].candidate.as_mut().unwrap().presentation = Some(CandidatePresentation {
        sort_order: Some(4),
        ..Default::default()
    });
    sort_candidates(&mut choices, CandidatesOrder::Custom);
    assert_eq!(choices[0].choice.id, "bea");
}

#[test]
fn qr_content_lists_selected_names_and_distinguishes_abstention() {
    let contest = contest();
    let bridges = convert_ballots(&election(), vec![ballot(&["ada"])]).unwrap();
    let mut data = vec![ContestData {
        decoded_choices: DecodedChoice::from_dvcs(&bridges[0].choices[0], &contest),
        contest,
        undervotes: 1,
        overvotes: 0,
        digital_signature: None,
        sign_data: None,
        page_number: Some(1),
    }];
    assert_eq!(qr_encode_choices(&data, "Receipt"), "Receipt:Council:Ada");
    data[0].decoded_choices[0].choice.selected = -1;
    assert!(data[0].is_blank());
    assert_eq!(qr_encode_choices(&data, "Receipt"), "Receipt:ABSTENTION");
    assert_eq!(qr_encode_choices(&vec![], "Receipt"), "Receipt:ABSTENTION");
}

#[test]
fn html_receipts_escape_names_and_can_disable_pdf_rendering() {
    let mut contest = contest();
    contest.candidates[0].name = Some("<Ada & Co>".into());
    let election = election();
    let ballots = convert_ballots(&election, vec![ballot(&["ada"])]).unwrap();
    let mut config = PipeConfigBallotImages::mcballot();
    config.enable_pdfs = false;
    config.extra_data = json!({"title":"Receipt"});
    config.template = "{{#each data.ballot_data}}{{encoded_vote}}{{/each}}".into();
    config.system_template = "<main>{{{rendered_user_template}}}</main>".into();
    let (pdf, html) = image_pipe()
        .print_ballot_images(&ballots, &vec![contest], &election, &config, "North")
        .unwrap();
    assert!(pdf.is_none());
    assert_eq!(
        String::from_utf8(html).unwrap(),
        "<main>Receipt:Council:&lt;Ada &amp; Co&gt;</main>"
    );

    config.template = "{{#if}}".into();
    assert!(image_pipe()
        .print_ballot_images(
            &ballots,
            &vec![self::contest()],
            &election,
            &config,
            "North"
        )
        .is_err());
}

#[tokio::test]
async fn manifest_csv_quotes_special_characters_and_replaces_old_content() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("manifest.csv");
    let records = vec![BallotCsvData {
        file_name: "ballot,\"north\".pdf".into(),
        hash: "digest".into(),
    }];
    write_file_hash_csv(records, path.clone()).await.unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "file_name,hash\n\"ballot,\"\"north\"\".pdf\",digest\n"
    );

    write_file_hash_csv(vec![], path.clone()).await.unwrap();
    assert_eq!(fs::read_to_string(path).unwrap(), "file_name,hash\n");
    assert!(write_file_hash_csv(vec![], directory.path().to_path_buf())
        .await
        .is_err());
}

/// A single area keeps the real file-to-PDF path cheap. The template has no
/// network assets, so a disconnected worker can still verify every artifact.
fn file_image_pipe(root: &Path, ballots: &[DecodedBallotChoices]) -> MCBallotImages {
    use crate::config::PipeConfig;
    use crate::pipes::pipe_inputs::{AreaConfig, InputAreaConfig};
    use sequent_core::types::templates::ReportOptions;

    let mut pipe = image_pipe();
    pipe.pipe_inputs.cli.output_dir = root.into();
    let mut election = election();
    let area_id = Uuid::from_u128(3);
    let area = AreaConfig {
        id: area_id,
        name: "North".into(),
        tenant_id: Uuid::from_u128(4),
        election_event_id: Uuid::from_u128(5),
        election_id: election.id,
        census: 10,
        parent_id: None,
        auditable_votes: 2,
        votes_by_channel: None,
    };
    election.areas = vec![(&area).into()];
    election.areas[0].annotations = Some(json!({"miru:area-station-id": "STATION"}));
    election.annotations = HashMap::from([
        ("miru:precinct-code".into(), "POST".into()),
        ("clustered_precint_id".into(), "CLUSTER".into()),
    ]);
    election.contest_list[0].area_list = vec![InputAreaConfig {
        id: area_id,
        election_id: election.id,
        contest_id: Uuid::from_u128(1),
        census: 10,
        auditable_votes: 2,
        path: root.into(),
        area,
    }];

    let input = PipeInputs::mcballots_path(
        &root.join(PipeNameOutputDir::DecodeMCBallots.as_ref()),
        &election.id,
        &area_id,
    );
    fs::create_dir_all(&input).unwrap();
    fs::write(
        input.join(OUTPUT_DECODED_BALLOTS_FILE),
        serde_json::to_vec(ballots).unwrap(),
    )
    .unwrap();

    let mut config = PipeConfigBallotImages::mcballot();
    config.template = "{{#each data.ballot_data}}<p>{{encoded_vote}}</p>{{/each}}".into();
    config.system_template =
        "<!doctype html><html><body>{{{rendered_user_template}}}</body></html>".into();
    config.extra_data = json!({"title": "Receipt"});
    config.report_options = Some(ReportOptions {
        max_items_per_report: Some(2),
        max_threads: Some(1),
    });
    pipe.pipe_inputs.stage.current_pipe = Some(PipeName::MCBallotImages);
    pipe.pipe_inputs.stage.pipeline = vec![PipeConfig {
        id: PIPE_ID.into(),
        pipe: PipeName::MCBallotImages,
        config: Some(serde_json::to_value(config).unwrap()),
    }];
    pipe.pipe_inputs.election_list = vec![election];
    pipe
}

fn image_output(root: &Path) -> PathBuf {
    PipeInputs::mcballots_path(
        &root.join(PipeNameOutputDir::MCBallotImages.as_ref()),
        &Uuid::from_u128(2),
        &Uuid::from_u128(3),
    )
}

#[test]
fn pdf_batches_publish_matching_html_serial_ranges_and_a_complete_manifest() {
    let directory = tempdir().unwrap();
    let ballots: Vec<_> = [1, 2, 3]
        .into_iter()
        .map(|serial| {
            let mut ballot = ballot(&["ada"]);
            ballot.serial_number = Some(format!("{serial:09}"));
            ballot
        })
        .collect();
    let pipe = file_image_pipe(directory.path(), &ballots);
    pipe.exec().unwrap();

    // Three ballots at two per batch must produce exactly two PDFs. Check the
    // manifest against actual bytes, not just the existence of output files.
    let output = image_output(directory.path());
    let mut reader = csv::Reader::from_path(output.join("ballots_files.csv")).unwrap();
    assert_eq!(
        reader.headers().unwrap().iter().collect::<Vec<_>>(),
        ["file_name", "hash"]
    );
    let mut rows = reader
        .records()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    rows.sort_by(|left, right| left[0].cmp(&right[0]));
    assert_eq!(rows.len(), 2);
    for (row, range) in rows
        .iter()
        .zip(["000000001-000000002", "000000003-000000003"])
    {
        let bytes = fs::read(output.join(&row[0])).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        let digest = hex::encode(hash_sha256(&bytes).unwrap());
        assert_eq!(
            row[0],
            format!("ballots_POST_STATION_CLUSTER_{range}_{digest}.pdf")
        );
        // The existing manifest contract hashes the filename, whose suffix
        // carries the content digest. Assert both links in that chain.
        assert_eq!(row[1], hash_b64(row[0].as_bytes()).unwrap());
    }
    for (batch, expected) in [(0, 2), (1, 1)] {
        let html = fs::read_to_string(output.join(format!("ballots_batch-{batch}.html"))).unwrap();
        assert_eq!(html.matches("Receipt:Council:Ada").count(), expected);
    }
}

#[test]
fn an_empty_area_still_produces_a_pdf_with_the_empty_serial_range() {
    let directory = tempdir().unwrap();
    file_image_pipe(directory.path(), &[]).exec().unwrap();
    let output = image_output(directory.path());
    let mut reader = csv::Reader::from_path(output.join("ballots_files.csv")).unwrap();
    let rows = reader
        .records()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0][0].contains("_000000000-000000000_"));
    assert!(fs::read(output.join(&rows[0][0]))
        .unwrap()
        .starts_with(b"%PDF-"));
    assert!(output.join("ballots_batch-0.html").exists());
}

#[test]
fn invalid_receipt_templates_fail_the_pipe_instead_of_reporting_success() {
    let directory = tempdir().unwrap();
    let mut pipe = file_image_pipe(directory.path(), &[ballot(&["ada"])]);
    let config = pipe.pipe_inputs.stage.pipeline[0].config.as_mut().unwrap();
    config["template"] = json!("{{#if}}");
    config["enable_pdfs"] = json!(false);
    assert!(
        pipe.exec().is_err(),
        "missing receipts must fail the pipeline"
    );
    assert!(!image_output(directory.path())
        .join("ballots_files.csv")
        .exists());
}

#[test]
fn zero_batch_size_is_a_configuration_error_instead_of_a_panic() {
    let directory = tempdir().unwrap();
    let mut pipe = file_image_pipe(directory.path(), &[ballot(&["ada"])]);
    let config = pipe.pipe_inputs.stage.pipeline[0].config.as_mut().unwrap();
    config["report_options"]["max_items_per_report"] = json!(0);
    config["enable_pdfs"] = json!(false);
    assert!(pipe.exec().is_err());
}
