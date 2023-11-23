// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct User {
    pub id: Option<String>,
    pub attributes: Option<HashMap<String, Value>>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub enabled: Option<bool>,
    pub first_name: Option<String>,
    pub groups: Option<Vec<String>>,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct Permission {
    pub id: Option<String>,
    pub attributes: Option<HashMap<String, Value>>,
    pub container_id: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
}
