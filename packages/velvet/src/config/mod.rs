use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::pipes::{deserialize_pipe, PipeName};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    version: String,
    stages: Stages,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Stages {
    order: Vec<String>,
    #[serde(flatten)]
    stages: HashMap<String, Stage>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Stage {
    pipeline: Vec<Pipeline>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Pipeline {
    id: String,
    #[serde(deserialize_with = "deserialize_pipe")]
    pipe: PipeName,
    config: Option<serde_json::Value>,
}
