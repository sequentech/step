// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Read only election structure from a platform event export. Credentials,
//! voters, ballot contents, realm secrets, and unrelated ZIP members are ignored.
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read},
};
const MAX_BYTES: usize = 24_000_000;
fn rows<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    let rows = value[key]
        .as_array()
        .ok_or_else(|| format!("Election export requires {key}"))?;
    if rows.len() > 20_000 {
        return Err(format!("Too many {key}"));
    }
    Ok(rows)
}
fn id(value: &Value, key: &str) -> Result<String, String> {
    let id = value[key]
        .as_str()
        .filter(|v| !v.is_empty() && v.len() <= 128)
        .ok_or_else(|| format!("Missing {key}"))?;
    Ok(id.into())
}
fn public_record(value: &Value) -> Value {
    let mut result = serde_json::Map::new();
    for key in [
        "id",
        "name",
        "description",
        "alias",
        "external_id",
        "election_id",
        "contest_id",
        "min_votes",
        "max_votes",
        "is_explicit_invalid",
        "is_explicit_blank",
    ] {
        if let Some(value) = value.get(key) {
            result.insert(key.into(), value.clone());
        }
    }
    // Platform display names are stored in presentation; old exports may also
    // carry a top-level name. Keep both forms usable without leaking metadata.
    if !result.contains_key("name") {
        result.insert(
            "name".into(),
            value
                .pointer("/presentation/name")
                .cloned()
                .unwrap_or_else(|| value["description"].clone()),
        );
    }
    if let Some(alias) = value.pointer("/presentation/alias") {
        result.insert("alias".into(), alias.clone());
    }
    // Labels are useful for multilingual templates; avoid unrelated presentation metadata.
    if let Some(i18n) = value
        .pointer("/presentation/i18n")
        .or_else(|| value.get("i18n"))
    {
        result.insert("i18n".into(), i18n.clone());
    }
    Value::Object(result)
}
pub fn from_export(value: &Value) -> Result<Value, String> {
    if value.to_string().len() > MAX_BYTES {
        return Err("Election JSON exceeds 24 MB".into());
    }
    let event = &value["election_event"];
    id(event, "id")?;
    let elections = rows(value, "elections")?;
    let contests = rows(value, "contests")?;
    let candidates = rows(value, "candidates")?;
    let mut seen = BTreeSet::new();
    for row in elections.iter().chain(contests).chain(candidates) {
        if !seen.insert(id(row, "id")?) {
            return Err("Duplicate election, contest or candidate ID".into());
        }
    }
    let election_ids: BTreeSet<_> = elections
        .iter()
        .map(|e| id(e, "id"))
        .collect::<Result<_, _>>()?;
    let contest_ids: BTreeSet<_> = contests
        .iter()
        .map(|c| id(c, "id"))
        .collect::<Result<_, _>>()?;
    let mut by_contest: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for candidate in candidates {
        let contest = id(candidate, "contest_id")?;
        if !contest_ids.contains(&contest) {
            return Err("Candidate references a missing contest".into());
        }
        by_contest
            .entry(contest)
            .or_default()
            .push(public_record(candidate));
    }
    let mut clean_contests = Vec::new();
    for contest in contests {
        if !election_ids.contains(&id(contest, "election_id")?) {
            return Err("Contest references a missing election".into());
        }
        let mut clean = public_record(contest);
        let mut children = by_contest.remove(&id(contest, "id")?).unwrap_or_default();
        // Export array order is preserved: ballot order is meaningful.
        clean["candidates"] = json!(children.drain(..).collect::<Vec<_>>());
        clean_contests.push(clean);
    }
    Ok(
        json!({"version":1,"kind":"sequent-election-sample-data","knownData":{
        "event":public_record(event),"elections":elections.iter().map(public_record).collect::<Vec<_>>(),
        "contests":clean_contests,"candidates":candidates.iter().map(public_record).collect::<Vec<_>>()
    },"counts":{"elections":elections.len(),"contests":contests.len(),"candidates":candidates.len()},
    "warnings":["Only election structure was imported. Voters, credentials, votes and results were not imported."]}),
    )
}
pub fn import(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() > MAX_BYTES {
        return Err("Sample data upload exceeds 24 MB".into());
    }
    let value: Value = if bytes.starts_with(b"PK") {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
        if zip.len() > 20_000 {
            return Err("Too many ZIP members".into());
        }
        let names: Vec<_> = zip
            .file_names()
            .filter(|name| {
                let base = name.rsplit('/').next().unwrap_or(name);
                base.starts_with("export_election_event-") && base.ends_with(".json")
            })
            .map(str::to_owned)
            .collect();
        if names.len() != 1 {
            return Err("ZIP must contain exactly one export_election_event-*.json".into());
        }
        let file = zip.by_name(&names[0]).map_err(|e| e.to_string())?;
        if file.size() > MAX_BYTES as u64 {
            return Err("Election JSON exceeds 24 MB".into());
        }
        let mut json_bytes = Vec::new();
        file.take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut json_bytes)
            .map_err(|e| e.to_string())?;
        if json_bytes.len() > MAX_BYTES {
            return Err("Election JSON exceeds 24 MB".into());
        }
        serde_json::from_slice(&json_bytes).map_err(|e| e.to_string())?
    } else {
        serde_json::from_slice(bytes).map_err(|e| e.to_string())?
    };
    if value["kind"] == "sequent-election-sample-data" && value["version"] == 1 {
        // Round-trip through the same allowlist/relationship validation.
        let known = &value["knownData"];
        from_export(
            &json!({"election_event":known["event"],"elections":known["elections"],"contests":known["contests"],"candidates":known["candidates"]}),
        )
    } else {
        from_export(&value)
    }
}
