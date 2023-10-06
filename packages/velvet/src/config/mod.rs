use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pipe: String,
    config: Option<HashMap<String, String>>,
    formats: Option<Vec<String>>,
}
