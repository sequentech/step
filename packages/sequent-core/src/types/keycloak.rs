// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;


#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct UserArea {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct User {
    pub id: Option<String>,
    pub attributes: Option<HashMap<String, Value>>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub enabled: Option<bool>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub area: Option<UserArea>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct Permission {
    pub id: Option<String>,
    pub attributes: Option<HashMap<String, Value>>,
    pub container_id: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct Role {
    pub id: Option<String>,
    pub name: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub access: Option<HashMap<String, Value>>,
    pub attributes: Option<HashMap<String, Value>>,
    pub client_roles: Option<HashMap<String, Value>>,
}
