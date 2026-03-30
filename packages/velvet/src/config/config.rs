// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Configuration types and helpers for Velvet pipelines.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::pipes::pipe_name::{deserialize_pipe, PipeName};

/// Top-level configuration for a Velvet election pipeline.
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    /// Configuration version identifier.
    pub version: String,
    /// Pipeline stages and their execution order.
    pub stages: Stages,
}

/// Pipeline stages definition and execution order.
#[derive(Serialize, Deserialize, Debug)]
pub struct Stages {
    /// Ordered list of stage names to execute.
    pub order: Vec<String>,
    /// Mapping of stage names to their pipeline configurations.
    #[serde(flatten)]
    pub stages_def: HashMap<String, Stage>,
}

/// A single pipeline stage containing multiple pipes.
#[derive(Serialize, Deserialize, Debug)]
pub struct Stage {
    /// Ordered list of pipes to execute in this stage.
    pub pipeline: Vec<PipeConfig>,
}

/// Configuration for a single pipeline component (pipe).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PipeConfig {
    /// Unique identifier for this pipe instance.
    pub id: String,
    /// Type of pipe to execute.
    #[serde(deserialize_with = "deserialize_pipe")]
    pub pipe: PipeName,
    /// Pipe-specific configuration data.
    pub config: Option<serde_json::Value>,
}
