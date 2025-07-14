// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use tracing::instrument;

use crate::pipes::pipe_inputs::PipeInputs;
use crate::pipes::Pipe;
use std::path::PathBuf;
use crate::pipes::pipe_name::PipeNameOutputDir;

#[derive(Debug)]
pub struct GenerateDatabase {
    pub pipe_inputs: PipeInputs,
    pub input_dir: PathBuf,
    pub output_dir: PathBuf,
}

impl Pipe for GenerateDatabase {
    #[instrument(err, skip_all, name = "GenerateDatabase::exec")]
    fn exec(&self) -> crate::pipes::error::Result<()> {
        todo!()
    }
}

impl GenerateDatabase {
    #[instrument(skip_all, name = "GenerateDatabase::new")]
    pub fn new(pipe_inputs: PipeInputs) -> Self {
        let input_dir = pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::MarkWinners.as_ref());
        let output_dir = pipe_inputs
            .cli
            .output_dir
            .as_path()
            .join(PipeNameOutputDir::GenerateReports.as_ref());

        Self {
            pipe_inputs,
            input_dir,
            output_dir,
        }
    }
}