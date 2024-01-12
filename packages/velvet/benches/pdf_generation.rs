#![feature(test)]

extern crate test;
use test::Bencher;
use velvet::{
    cli::{state::State, test_all::generate_ballots, CliRun},
    fixtures::TestFixture,
    pipes::PipeManager,
};

#[bench]
fn bench_pdf_generation(b: &mut Bencher) {
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

    b.iter(|| {
        // Generate reports
        pm.exec().expect("Failed to execute pipe");
    });
}
