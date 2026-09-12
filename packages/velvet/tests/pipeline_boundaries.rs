// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Pipeline configuration controls which transformations run on the ballots.
//! Reject inconsistent definitions and make the beginning/end states explicit.

use clap::Parser;
use serde_json::{json, Value};
use std::fs;
use tempfile::tempdir;
use velvet::cli::{state::Stage, Cli, CliRun};
use velvet::config::{Config, PipeConfig};
use velvet::pipes::pipe_inputs::{PipeInputs, DEFAULT_DIR_CONFIGS};
use velvet::pipes::pipe_name::PipeName;

fn configuration() -> Value {
    json!({
        "version": "1.0.0",
        "stages": {
            "order": ["main"],
            "main": { "pipeline": [
                {"id": "decode", "pipe": "VelvetDecodeBallots"},
                {"id": "tally", "pipe": "VelvetDoTally"},
                {"id": "winners", "pipe": "VelvetMarkWinners"}
            ]}
        }
    })
}

#[test]
fn configuration_requires_defined_stages_and_unique_transformations() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("pipeline.json");
    let cli = CliRun {
        stage: "main".into(),
        pipe_id: "tally".into(),
        config: path.clone(),
        input_dir: directory.path().join("input"),
        output_dir: directory.path().join("output"),
    };
    assert!(cli.validate().is_err()); // Missing file must not mean default settings.
    let valid = configuration();
    fs::write(&path, serde_json::to_vec(&valid).unwrap()).unwrap();
    assert!(cli.validate().is_ok());

    let mut missing_stage = valid.clone();
    missing_stage["stages"]["order"] = json!(["absent"]);
    let mut duplicate_pipe = valid.clone();
    duplicate_pipe["stages"]["main"]["pipeline"][2]["pipe"] = json!("VelvetDoTally");
    let mut unknown_pipe = valid.clone();
    unknown_pipe["stages"]["main"]["pipeline"][1]["pipe"] = json!("VelvetDoesNotExist");
    let mut wrong_type = valid;
    wrong_type["stages"]["main"]["pipeline"][0]["pipe"] = json!(42);

    for invalid in [missing_stage, duplicate_pipe, unknown_pipe, wrong_type] {
        fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
        assert!(cli.validate().is_err(), "must reject {invalid}");
    }
    fs::write(&path, b"{truncated").unwrap();
    assert!(cli.validate().is_err());
}

#[test]
fn pipeline_neighbors_stop_at_both_ends_and_preserve_per_pipe_settings() {
    let config: Config = serde_json::from_value(configuration()).unwrap();
    let mut pipeline = config.stages.stages_def["main"].pipeline.clone();
    pipeline[1].config = Some(json!({"audit": true}));
    let mut stage = Stage {
        name: "main".into(),
        pipeline,
        current_pipe: Some(PipeName::DecodeBallots),
        previous_pipe: None,
    };
    assert_eq!(stage.previous_pipe(), None);
    assert_eq!(stage.next_pipe(), Some(PipeName::DoTally));
    stage.current_pipe = Some(PipeName::DoTally);
    assert_eq!(stage.previous_pipe(), Some(PipeName::DecodeBallots));
    assert_eq!(stage.next_pipe(), Some(PipeName::MarkWinners));
    assert_eq!(
        stage.pipe_config(stage.current_pipe).unwrap().config,
        Some(json!({"audit": true}))
    );
    stage.current_pipe = Some(PipeName::MarkWinners);
    assert_eq!(stage.next_pipe(), None);
    stage.current_pipe = None;
    assert_eq!(stage.previous_pipe(), Some(PipeName::MarkWinners));
    assert_eq!(stage.next_pipe(), None);
    assert!(stage.pipe_config(None).is_none());
    assert!(stage
        .pipe_config(Some(PipeName::GenerateDatabase))
        .is_none());

    stage.current_pipe = Some(PipeName::GenerateDatabase);
    assert_eq!(stage.previous_pipe(), None);
    assert_eq!(stage.next_pipe(), None);
}

#[test]
fn an_empty_stage_has_no_previous_or_next_pipe() {
    let stage = Stage {
        name: "empty".into(),
        pipeline: Vec::<PipeConfig>::new(),
        current_pipe: None,
        previous_pipe: None,
    };
    assert_eq!(stage.previous_pipe(), None);
    assert_eq!(stage.next_pipe(), None);
}

#[test]
fn malformed_election_folder_names_return_errors_without_panicking() {
    let directory = tempdir().unwrap();
    let configs = directory.path().join(DEFAULT_DIR_CONFIGS);
    fs::create_dir_all(&configs).unwrap();
    let cli = CliRun {
        stage: "main".into(),
        pipe_id: "tally".into(),
        config: directory.path().join("pipeline.json"),
        input_dir: directory.path().to_path_buf(),
        output_dir: directory.path().join("output"),
    };
    let load = || {
        PipeInputs::new(
            cli.clone(),
            Stage {
                name: "main".into(),
                pipeline: vec![],
                current_pipe: None,
                previous_pipe: None,
            },
        )
    };
    assert!(load().unwrap().election_list.is_empty());

    // The second name is long enough in bytes, but taking its final 36 bytes
    // would start inside a UTF-8 character. Length checks alone are insufficient.
    for name in [
        "election__short".to_string(),
        format!("election__é{}", "x".repeat(35)),
    ] {
        let malformed = configs.join(name);
        fs::create_dir(&malformed).unwrap();
        assert!(
            load().is_err(),
            "malformed input directory must be rejected"
        );
        fs::remove_dir(malformed).unwrap();
    }
}

#[test]
fn command_line_requires_an_explicit_stage_pipe_and_all_file_locations() {
    let valid = [
        "velvet",
        "run",
        "main",
        "tally",
        "--config",
        "pipeline.json",
        "--input-dir",
        "input",
        "--output-dir",
        "output",
    ];
    assert!(Cli::try_parse_from(valid).is_ok());
    for incomplete in [vec!["velvet"], vec!["velvet", "run"], valid[..8].to_vec()] {
        assert!(Cli::try_parse_from(incomplete).is_err());
    }
    let mut unknown = valid.to_vec();
    unknown.push("--skip-verification");
    assert!(Cli::try_parse_from(unknown).is_err());
}
