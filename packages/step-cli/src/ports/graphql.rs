// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use graphql_client::Response;
use serde::{de::DeserializeOwned, Serialize};
use std::error::Error;

/// Sends a GraphQL request and decodes the response body. How `data` and
/// `errors` combine into a result is left to each command.
pub trait GraphqlClient {
    fn post<T, B>(&self, request_body: &B) -> Result<Response<T>, Box<dyn Error>>
    where
        T: DeserializeOwned,
        B: Serialize + ?Sized;
}
