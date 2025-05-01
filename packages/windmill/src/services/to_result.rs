// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use graphql_client::Response;
use tracing::{event, instrument, Level};

pub trait ToResult<T, E> {
    fn ok(self) -> Result<T, E>;
}

impl<T> ToResult<Response<T>, anyhow::Error> for Response<T> {
    #[instrument(skip_all)]
    fn ok(self) -> Result<Response<T>, anyhow::Error> {
        if self.errors.is_some() {
            let messages = self
                .errors
                .clone()
                .ok_or(anyhow!("Unexpected: empty errors list"))?
                .into_iter()
                .map(|error| error.message.clone())
                .collect::<Vec<String>>()
                .join(" - ");
            event!(Level::ERROR, "response errors: {}", messages);
            Err(anyhow!(messages))
        } else {
            Ok(self)
        }
    }
}
