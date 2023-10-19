#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::state::State;
    use crate::cli::CliRun;
    use crate::fixtures::TestFixture;
    use crate::pipes::decode_ballots::ballot_codec::BallotCodec;
    use crate::pipes::decode_ballots::OUTPUT_DECODED_BALLOTS_FILE;
    use anyhow::{Error, Result};
    use rand::Rng;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use walkdir::WalkDir;

    fn generate_ballots(
        fixture: &TestFixture,
        election_num: u32,
        contest_num: u32,
        ballots_num: u32,
    ) -> Result<()> {
        let bases = vec![2, 2, 2, 2, 2, 2];
        let ballot_codec = BallotCodec::new(bases.clone());
        let mut rng = rand::thread_rng();

        (0..election_num).try_for_each(|_| {
            let uuid_election = fixture.create_election_config()?;
            (0..contest_num).try_for_each(|_| {
                let uuid_contest = fixture.create_contest_config(&uuid_election)?;

                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(format!(
                        "{}/election__{}/contest__{}/ballots.csv",
                        fixture.input_dir_ballots, uuid_election, uuid_contest
                    ))?;
                (0..ballots_num).try_for_each(|_| {
                    let mut choices = bases.clone().iter_mut().map(|_| 0).collect::<Vec<u32>>();
                    let index = rng.gen_range(0..choices.len());
                    choices[index] = 1;

                    let encoded_ballot = ballot_codec.encode_ballot(choices.clone());
                    writeln!(file, "{}", encoded_ballot)?;

                    Ok::<(), Error>(())
                })?;
                Ok::<(), Error>(())
            })?;
            Ok::<(), Error>(())
        })?;

        Ok(())
    }

    #[test]
    fn test_create_configs() -> Result<()> {
        let fixture = TestFixture::new()?;

        let uuid_election = fixture.create_election_config()?;
        let uuid_contest = fixture.create_contest_config(&uuid_election)?;

        let entries = fs::read_dir(&fixture.input_dir_configs)?;
        let count = entries.count();
        assert_eq!(count, 1);

        let entries = fs::read_dir(format!(
            "{}/election__{}",
            &fixture.input_dir_configs, uuid_election
        ))?;
        let count = entries.count();
        assert_eq!(count, 2);

        let entries = fs::read_dir(format!(
            "{}/election__{}/contest__{}",
            &fixture.input_dir_configs, uuid_election, uuid_contest
        ))?;
        let count = entries.count();
        assert_eq!(count, 1);

        Ok(())
    }

    #[test]
    fn test_ballot_codec() {
        let choices = vec![0, 0, 0, 1, 0, 0];
        let ballot_codec = BallotCodec::new(vec![2, 2, 2, 2, 2, 2]);
        let encoded_ballot = ballot_codec.encode_ballot(choices.clone());
        let decoded_ballot = ballot_codec.decode_ballot(encoded_ballot);

        assert_eq!(decoded_ballot, choices);
    }
    #[test]
    fn test_create_ballots() -> Result<()> {
        let fixture = TestFixture::new()?;

        generate_ballots(&fixture, 5, 10, 5)?;

        // count elections
        let entries = fs::read_dir(&fixture.input_dir_ballots)?;
        let count = entries.count();
        assert_eq!(count, 5);

        // count contests
        let mut entries = fs::read_dir(&fixture.input_dir_ballots)?;
        let entry = entries.next().unwrap()?;
        let contest_path = entry.path();
        let entries = fs::read_dir(contest_path)?;
        let count = entries.count();
        assert_eq!(count, 10);

        Ok(())
    }

    #[test]
    fn test_decode_ballots() -> Result<()> {
        let election_num = 5;
        let contest_num = 10;
        let fixture = TestFixture::new()?;

        generate_ballots(&fixture, 5, 10, 20)?;

        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "decode-ballots".to_string(),
            config: fixture.config_path.clone(),
            input_dir: PathBuf::from(format!("{}/tests/input-dir", &fixture.root_dir)),
            output_dir: PathBuf::from(format!("{}/tests/output-dir", &fixture.root_dir)),
        };

        let config = cli.parse_config()?;
        let mut state = State::new(&cli, &config)?;

        // DecodeBallots
        state.exec_next(&cli.stage)?;

        assert_eq!(
            WalkDir::new(cli.output_dir)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path()
                        .file_name()
                        .map_or(false, |f| f == OUTPUT_DECODED_BALLOTS_FILE)
                })
                .count(),
            election_num * contest_num
        );

        // DoTally
        state.exec_next(&cli.stage)?;

        Ok(())
    }
}
