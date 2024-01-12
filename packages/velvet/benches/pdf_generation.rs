#![feature(test)]

extern crate test;
use anyhow::Result;
use test::Bencher;
use velvet::{
    cli::{state::State, test_all::generate_ballots, CliRun},
    fixtures::TestFixture,
    pipes::PipeManager,
};

#[bench]
fn bench_pdf_generation(b: &mut Bencher) -> Result<()> {
    let fixture = TestFixture::new()?;

    generate_ballots(&fixture, 1, 2, 1, 50)?;

    let cli = CliRun {
        stage: "main".to_string(),
        pipe_id: "decode-ballots".to_string(),
        config: fixture.config_path.clone(),
        input_dir: fixture.root_dir.join("tests").join("input-dir"),
        output_dir: fixture.root_dir.join("tests").join("output-dir"),
    };

    let config = cli.validate()?;
    let mut state = State::new(&cli, &config)?;

    // DecodeBallots
    state.exec_next()?;

    // DoTally
    state.exec_next()?;

    // MarkWinners
    state.exec_next()?;

    let stage = state
        .stages
        .iter()
        .find(|s| s.name == "main")
        .unwrap();
    dbg!(&state.stages);
    println!("sdfsadfdsafsadfsadfsaf");
    dbg!(&stage);
    let cli = state.cli.clone();
    let pm = PipeManager::get_pipe(cli, stage.clone()).unwrap().unwrap();

    b.iter(|| {
        // Generate reports
        pm.exec().unwrap();
    });

    Ok(())
}
