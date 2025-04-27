// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use csv::StringRecord;
use ordered_float::NotNan;
use sequent_core::services::date::ISO8601;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{info, instrument};
use uuid::Uuid;

#[instrument(err, skip_all)]
pub async fn process_uuids(
    ids: Option<&str>,
    replacement_map: HashMap<String, String>,
) -> Result<Option<Vec<String>>> {
    match ids {
        None => Ok(None),
        Some(ids) => {
            let parsed: Vec<String> = serde_json::from_str::<Vec<String>>(ids)
                .map_err(|e| anyhow!("Failed to parse UUID array as JSON: {:?}", e))?;

            let new_ids: Vec<String> = parsed
                .into_iter()
                .map(|id| {
                    replacement_map
                        .get(&id)
                        .cloned()
                        .ok_or_else(|| anyhow!("Can't find id: {id} in replacement map"))
                })
                .collect::<Result<_>>()?;

            Ok(Some(new_ids))
        }
    }
}

#[instrument(err, skip_all)]
pub async fn get_replaced_id(
    record: &StringRecord,
    index: i32,
    replacement_map: &HashMap<String, String>,
) -> Result<String> {
    let id: String = record
        .get(index as usize)
        .ok_or_else(|| anyhow!("Missing column {index}"))
        .and_then(|s| serde_json::from_str(s).map_err(|e| anyhow!("Invalid JSON: {:?}", e)))?;
    let new_id = replacement_map
        .get(&id)
        .ok_or(anyhow!("Can't find id:{id} in replacement map"))?
        .clone();

    Ok(new_id)
}

#[instrument(err, skip_all)]
pub async fn get_opt_i64_item(record: &StringRecord, index: usize) -> Result<Option<i64>> {
    let item = record
        .get(index)
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "null")
        .map(|s| s.parse::<i64>())
        .transpose()
        .map_err(|err| anyhow!("Error parsing as i64 at column {index}: {:?}", err))?;
    Ok(item)
}

#[instrument(err, skip_all)]
pub async fn get_opt_json_value_item(record: &StringRecord, index: usize) -> Result<Option<Value>> {
    let item = record
        .get(index)
        .filter(|s| !s.is_empty())
        .map(serde_json::from_str)
        .transpose()
        .map_err(|err| anyhow!("Error process json column {index} {:?}", err))?;
    Ok(item)
}

#[instrument(err, skip_all)]
pub async fn get_opt_f64_item(record: &StringRecord, index: usize) -> Result<Option<NotNan<f64>>> {
    let item = record
        .get(index)
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "null")
        .map(|s| {
            let value = s
                .parse::<f64>()
                .map_err(|e| anyhow!("Error parsing as f64 at column {index}: {:?}", e))?;
            NotNan::new(value).map_err(|e| anyhow!("Value is NaN (not allowed in NotNan): {:?}", e))
        })
        .transpose()?;
    Ok(item)
}

#[instrument(err, skip_all)]
pub async fn get_string_or_null_item(
    record: &StringRecord,
    index: usize,
) -> Result<Option<String>> {
    let item = record
        .get(index)
        .map(str::trim)
        .map(|s| {
            if s == "null" {
                Ok(None)
            } else {
                serde_json::from_str::<String>(s).map(Some)
            }
        })
        .transpose()
        .map_err(|err| anyhow!("Error at column {index}: {:?}", err))?
        .flatten();
    Ok(item)
}

#[instrument(err, skip_all)]
pub async fn get_opt_date(record: &StringRecord, index: usize) -> Result<Option<DateTime<Local>>> {
    let item = record
        .get(index)
        .map(|s| {
            let s = s.trim_matches('"');
            ISO8601::to_date(s).ok()
        })
        .flatten();
    Ok(item)
}
