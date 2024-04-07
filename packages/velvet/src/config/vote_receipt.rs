use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct PipeConfigVoteReceipts {
    pub template: String,
    pub extra_data: Value,
}

impl PipeConfigVoteReceipts {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for PipeConfigVoteReceipts {
    fn default() -> Self {
        let html = include_str!("../resources/vote_receipts.hbs");

        Self {
            template: html.to_string(),
            extra_data: json!("{}"),
        }
    }
}
