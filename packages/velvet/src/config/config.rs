// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::pipes::pipe_name::{deserialize_pipe, PipeName};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub version: String,
    pub stages: Stages,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Stages {
    pub order: Vec<String>,
    #[serde(flatten)]
    pub stages_def: HashMap<String, Stage>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Stage {
    pub pipeline: Vec<PipeConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PipeConfig {
    pub id: String,
    #[serde(deserialize_with = "deserialize_pipe")]
    pub pipe: PipeName,
    pub config: Option<serde_json::Value>,
}
