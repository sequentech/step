// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]

use serde::{Deserialize, Serialize};
use std::default::Default;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ResultDocuments {
    pub json: Option<String>,
    pub pdf: Option<String>,
    pub html: Option<String>,
}
