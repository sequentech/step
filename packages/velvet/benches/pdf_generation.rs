// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use velvet::{
    cli::{state::State, test_all::generate_ballots, CliRun},
    fixtures::TestFixture,
    pipes::PipeManager,
};

pub fn criterion_benchmark(c: &mut Criterion) {
    let fixture = TestFixture::new().expect("Failed to create test fixture");

    generate_ballots(&fixture, 1, 2, 1, 50).expect("Failed to generate ballots");

    let cli = CliRun {
        stage: "main".to_string(),
        pipe_id: "decode-ballots".to_string(),
        config: fixture.config_path.clone(),
        input_dir: fixture.root_dir.join("tests").join("input-dir"),
        output_dir: fixture.root_dir.join("tests").join("output-dir"),
    };

    let config = cli.validate().expect("Failed to validate CLI");
    let mut state = State::new(&cli, &config).expect("Failed to create state");

    // DecodeBallots
    state.exec_next().expect("Failed to exec DecodeBallots");

    // DoTally
    state.exec_next().expect("Failed to exec DoTally");

    // MarkWinners
    state.exec_next().expect("Failed to exec MarkWinners");

    let stage = state
        .stages
        .iter()
        .find(|s| s.name == "main")
        .expect("Failed to find 'main' stage");

    let cli = state.cli.clone();
    let pm = PipeManager::get_pipe(cli, stage.clone())
        .expect("Failed to get pipe")
        .expect("Failed to unwrap pipe");

    // Benchmarking

    let mut group = c.benchmark_group("generate_reports_group");

    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(75));
    group.sample_size(10);

    group.bench_function("generate reports", |b| {
        b.iter(|| {
            // Generate reports
            pm.exec().expect("Failed to execute pipe");
        })
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
