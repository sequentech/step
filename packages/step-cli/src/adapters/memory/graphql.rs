// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::graphql::GraphqlClient;
use graphql_client::Response;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::error::Error;
use std::sync::Mutex;

/// Answers each request with the next queued response body, or fails it as
/// the transport would, and records the JSON body of every request.
#[derive(Debug, Default)]
pub struct QueuedGraphql {
    responses: Mutex<VecDeque<Result<Value, String>>>,
    requests: Mutex<Vec<Value>>,
}

impl QueuedGraphql {
    pub fn respond(self, body: Value) -> Self {
        self.responses
            .lock()
            .expect("responses lock")
            .push_back(Ok(body));
        self
    }

    pub fn fail(self, message: &str) -> Self {
        self.responses
            .lock()
            .expect("responses lock")
            .push_back(Err(message.to_string()));
        self
    }

    pub fn requests(&self) -> Vec<Value> {
        self.requests.lock().expect("requests lock").clone()
    }
}

impl GraphqlClient for QueuedGraphql {
    fn post<T, B>(&self, request_body: &B) -> Result<Response<T>, Box<dyn Error>>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized,
    {
        let request = serde_json::to_value(request_body)?;
        self.requests
            .lock()
            .expect("requests lock")
            .push(request.clone());
        let response = self
            .responses
            .lock()
            .expect("responses lock")
            .pop_front()
            .unwrap_or_else(|| panic!("no response queued for {request}"));
        Ok(serde_json::from_value(response?)?)
    }
}
