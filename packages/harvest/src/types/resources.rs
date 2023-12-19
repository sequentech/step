// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Serialize, Deserialize, Debug)]
pub struct Aggregate {
    pub count: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TotalAggregate {
    pub aggregate: Aggregate,
}

// Enumeration for the valid order directions
#[derive(Debug, Deserialize, EnumString, Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum OrderDirection {
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DataList<T> {
    pub items: Vec<T>,
    pub total: TotalAggregate,
}
