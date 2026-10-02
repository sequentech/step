// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{fill_many, Prepared, DEFAULT_FONT};
use anyhow::{bail, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{Cursor, Seek, Write};
#[derive(Clone, Serialize, Deserialize)]
pub struct CachedPdf {
    #[serde(with = "pdf_bytes")]
    pub background: Vec<u8>,
    pub manifest: Prepared,
}
impl std::fmt::Debug for CachedPdf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedPdf")
            .field("background_bytes", &self.background.len())
            .field("fields", &self.manifest.fields.len())
            .finish()
    }
}
mod pdf_bytes {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        v: &Vec<u8>,
        s: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD.encode(v))
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        d: D,
    ) -> std::result::Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        if s.len() > 32_000_000 {
            return Err(serde::de::Error::custom("Background exceeds 24 MB"));
        }
        STANDARD.decode(s).map_err(serde::de::Error::custom)
    }
}
impl CachedPdf {
    pub fn fill<W: Write + Seek>(&self, data: &Value, output: W) -> Result<W> {
        if let Some(ballots) = data.pointer("/data/ballot_data").and_then(Value::as_array) {
            // Borrow the existing decoded batch; produce/drop one runtime map at
            // a time. No Vec<PDF>, no growing Document containing every page.
            // Clone only metadata: cloning the root first would transiently
            // duplicate the entire decoded ballot batch before dropping it.
            let mut shared = data
                .as_object()
                .unwrap()
                .iter()
                .filter(|(key, _)| key.as_str() != "data")
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<serde_json::Map<_, _>>();
            let shared_data = data["data"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(key, _)| key.as_str() != "ballot_data")
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<serde_json::Map<_, _>>();
            shared.insert("data".into(), Value::Object(shared_data));
            let shared = Value::Object(shared);
            fill_many(
                &self.background,
                &self.manifest,
                DEFAULT_FONT,
                ballots.iter().map(|ballot| {
                    let mut item = shared.clone();
                    item["ballot"] = ballot.clone();
                    let mut selections = serde_json::Map::new();
                    let mut contests = serde_json::Map::new();
                    for field in &self.manifest.fields {
                        if field.kind == "page" {
                            if let Some(id) = field.path.strip_prefix("/contests/") {
                                contests.insert(id.into(), json!(false));
                            }
                        }
                    }
                    for contest in ballot["contest_choices"].as_array().into_iter().flatten() {
                        if let Some(id) = contest.pointer("/contest/id").and_then(Value::as_str) {
                            contests.insert(id.into(), json!(true));
                        }
                        for candidate in contest
                            .pointer("/contest/candidates")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                        {
                            if let Some(id) = candidate["id"].as_str() {
                                selections.insert(id.into(), json!(false));
                            }
                        }
                        for choice in contest["decoded_choices"].as_array().into_iter().flatten() {
                            if let (Some(id), Some(selected)) = (
                                choice.pointer("/choice/id").and_then(Value::as_str),
                                choice.pointer("/choice/selected").and_then(Value::as_i64),
                            ) {
                                selections.insert(id.into(), json!(selected >= 0));
                            }
                        }
                    }
                    item["selections"] = Value::Object(selections);
                    item["contests"] = Value::Object(contests);
                    Ok(item)
                }),
                output,
            )
        } else {
            fill_many(
                &self.background,
                &self.manifest,
                DEFAULT_FONT,
                [Ok(result_fields(data, &self.manifest)?)],
                output,
            )
        }
    }
    /// Compatibility for existing report APIs that return bytes. Offline and
    /// large-batch callers should use fill() with a file-backed writer instead.
    pub fn bytes(&self, data: &Value) -> Result<Vec<u8>> {
        Ok(self.fill(data, Cursor::new(Vec::new()))?.into_inner())
    }
}
fn result_fields(data: &Value, manifest: &Prepared) -> Result<Value> {
    let mut result = data.clone();
    if let Some(reports) = data["reports"].as_array() {
        let mut flat = serde_json::Map::new();
        let mut scopes = serde_json::Map::new();
        let mut duplicate_scopes = std::collections::BTreeSet::new();
        let mut candidates = std::collections::BTreeMap::<String, Vec<(bool, Value)>>::new();
        let mut contests = serde_json::Map::new();
        for field in &manifest.fields {
            if field.kind == "page" {
                if let Some(id) = field.path.strip_prefix("/contests/") {
                    contests.insert(id.into(), json!(false));
                }
            }
        }
        for report in reports {
            let scope = report
                .pointer("/area/id")
                .and_then(Value::as_str)
                .unwrap_or("aggregate");
            let contest = report
                .pointer("/contest/id")
                .and_then(Value::as_str)
                .unwrap_or("election");
            if contest != "election" {
                contests.insert(contest.into(), json!(true));
            }
            let channel = report["channel_type"].as_str();
            let scope_key = match channel {
                Some(channel) => format!("{scope}:{contest}:{channel}"),
                None => format!("{scope}:{contest}"),
            };
            let mut rows = serde_json::Map::new();
            for candidate in report["candidate_result"].as_array().into_iter().flatten() {
                let Some(id) = candidate.pointer("/candidate/id").and_then(Value::as_str) else {
                    bail!("Result candidate is missing its stable ID");
                };
                let values = json!({"votes":candidate["total_count"],"percentage":candidate["percentage_votes"],"winning_position":candidate["winning_position"]});
                // Channel breakdowns share candidate IDs with their main totals.
                // Keep them addressable by scope without making the main result
                // ambiguous or accidentally substituting a partial count.
                if channel.is_none() {
                    candidates
                        .entry(id.into())
                        .or_default()
                        .push((scope == "aggregate", values.clone()));
                }
                rows.insert(id.into(), values);
            }
            if scopes
                .insert(scope_key.clone(), Value::Object(rows))
                .is_some()
            {
                duplicate_scopes.insert(scope_key);
            }
        }
        // Consolidated documents contain the aggregate and its area totals.
        // Prefer a unique aggregate; otherwise require one unambiguous area.
        for (id, values) in candidates {
            let aggregate = values
                .iter()
                .filter(|(aggregate, _)| *aggregate)
                .collect::<Vec<_>>();
            if aggregate.len() == 1 {
                flat.insert(id, aggregate[0].1.clone());
            } else if aggregate.is_empty() && values.len() == 1 {
                flat.insert(id, values[0].1.clone());
            }
        }
        // A template using raw /reports remains valid even if derived scopes
        // collide. Referencing an ambiguous scope still fails as a missing field.
        for scope in duplicate_scopes {
            scopes.remove(&scope);
        }
        result["results"] = Value::Object(flat);
        result["result_scopes"] = Value::Object(scopes);
        result["contests"] = Value::Object(contests);
    }
    Ok(result)
}
