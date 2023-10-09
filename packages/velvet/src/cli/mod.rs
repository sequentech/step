pub mod error;
pub mod state;

use crate::config::Config;

use self::error::{Error, Result};
use clap::{Parser, Subcommand};
use std::{fs::File, path::PathBuf};

#[derive(Parser)]
#[command(name = "Velvet")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Run(CliRun),
}

#[derive(Parser)]
pub struct CliRun {
    pub stage: String,
    pub pipe_id: String,

    #[arg(short, long)]
    pub config: PathBuf,

    #[arg(short, long)]
    pub input_dir: PathBuf,

    #[arg(short, long)]
    pub output_dir: PathBuf,
}

impl CliRun {
    pub fn validate(&self) -> Result<Config> {
        let config = self.parse_config()?;

        Ok(config)
    }

    fn parse_config(&self) -> Result<Config> {
        if !self.config.exists() {
            return Err(Error::ConfigNotFound);
        }

        let file = File::open(&self.config).map_err(|_| Error::CannotOpenConfig)?;
        let config: Config = serde_json::from_reader(file)?;

        for stage in &config.stages.order {
            if !config.stages.stages_def.contains_key(stage) {
                return Err(Error::StageDefinition(format!(
                    "Stage '{stage}', defined in stages.order, is not defined in stages."
                )));
            }
        }

        Ok(config)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::fs::OpenOptions;
    use std::io::Write;

    struct TestFixture {
        config_path: PathBuf,
    }

    impl TestFixture {
        fn new(filename: &str) -> Result<Self> {
            let config_path = PathBuf::from(filename);
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .open(&config_path)?;

            let config_content = r#"
                {
                    "version": "0.0.0",
                    "stages": {
                        "order": ["main"],
                        "main": {
                            "pipeline": [
                                {
                                    "id": "decode-ballots",
                                    "pipe": "VelvetDecodeBallots",
                                    "config": {}
                                },
                                {
                                    "id": "do-tally",
                                    "pipe": "VelvetDoTally",
                                    "config": {
                                        "invalidateVotes": "Fail"
                                    }
                                },
                                {
                                    "id": "consolidation",
                                    "pipe": "VelvetConsolidation",
                                    "config": {}
                                },
                                {
                                    "id": "ties-resolution",
                                    "pipe": "VelvetTiesResolution",
                                    "config": {}
                                },
                                {
                                    "id": "compute-result",
                                    "pipe": "VelvetComputeResult",
                                    "config": {}
                                },
                                {
                                    "id": "gen-report",
                                    "pipe": "VelvetGenerateReport",
                                    "config": {
                                        "formats": ["pdf", "csv"]
                                    }
                                }
                            ]
                        }
                    }
                }
            "#;

            writeln!(file, "{config_content}")?;

            Ok(TestFixture { config_path })
        }
    }

    impl Drop for TestFixture {
        fn drop(&mut self) {
            std::fs::remove_file(&self.config_path).unwrap();
        }
    }

    #[test]
    fn test_clirun_validate() -> Result<()> {
        let _fixture = TestFixture::new("test_clirun_validate_velvet-config.json");
        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "do-tally".to_string(),
            config: PathBuf::from("test_clirun_validate_velvet-config.json"),
            input_dir: PathBuf::new(),
            output_dir: PathBuf::new(),
        };

        let config = cli.validate()?;

        assert!(config.stages.stages_def.contains_key("main"));

        let main_stage = config.stages.stages_def.get("main").unwrap();
        assert_eq!(main_stage.pipeline.len(), 6);

        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_clirun_validate_found_not_found() {
        let _fixture = TestFixture::new("test_clirun_validate_found_not_found-velvet-config.json");
        let cli = CliRun {
            stage: "main".to_string(),
            pipe_id: "do-tally".to_string(),
            config: PathBuf::from("do-not-exist.json"),
            input_dir: PathBuf::new(),
            output_dir: PathBuf::new(),
        };

        let _ = cli.validate().unwrap();
    }
}
