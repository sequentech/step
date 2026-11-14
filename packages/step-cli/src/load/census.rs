// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Streaming census generation: one PBKDF2 hash per collision-group position,
//! unique exact usernames, and deterministic match-attribute values.
use super::{
    config::{Login, CENSUS_COLUMNS},
    files,
    input::Input,
};
use anyhow::{ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Datelike;
use ring::{
    pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
use sequent_core::services::keycloak::MULTIVALUE_USER_ATTRIBUTE_SEPARATOR;
use serde_json::{json, Value};
use std::{num::NonZeroU32, path::Path, time::Instant};

/// Attribute rendered by the realm as an HTML date input and compared as `YYYY-MM-DD`.
pub const DATE_OF_BIRTH: &str = "dateOfBirth";

/// Collision group of an absolute voter suffix. Mirrored by `voting-load/credentials.js`.
pub fn group(login: &Login, index: u64) -> u64 {
    index / login.voters_per_value as u64
}

/// Position inside the collision group, which selects the voter's password.
pub fn position(login: &Login, index: u64) -> u64 {
    index % login.voters_per_value as u64
}

/// Password suffix that keeps colliding voters distinguishable; empty without collisions.
pub fn password_suffix(login: &Login, index: u64) -> String {
    if login.voters_per_value == 1 {
        String::new()
    } else {
        format!("-{}", position(login, index))
    }
}

/// Deterministic attribute value shared by every voter of one collision group.
pub fn attribute_value(attribute: &str, group: u64) -> Result<String> {
    if attribute == DATE_OF_BIRTH {
        let origin = chrono::NaiveDate::from_ymd_opt(1900, 1, 1).context("Invalid date origin")?;
        let date = i64::try_from(group)
            .ok()
            .and_then(|days| origin.checked_add_signed(chrono::Duration::try_days(days)?))
            .filter(|date| date.year() <= 9999)
            .context("Date of birth outside the four-digit year range")?;
        Ok(date.format("%Y-%m-%d").to_string())
    } else {
        Ok(format!("g{group}"))
    }
}

/// Collision groups exercised by one voter range, for reports and census metadata.
pub fn distribution(login: &Login, start: u64, count: usize) -> Value {
    if login.username() {
        return json!({"match_attributes": [], "voters_per_value": 1, "values": count,
            "largest_group": 1, "max_candidates": login.max_candidates,
            "match_policy": login.match_policy, "exceeds_max_candidates": false});
    }
    let end = start + count as u64;
    let values = if count == 0 {
        0
    } else {
        group(login, end - 1) - group(login, start) + 1
    };
    let largest = login.voters_per_value.min(count);
    json!({
        "match_attributes": login.match_attributes,
        "voters_per_value": login.voters_per_value,
        "values": values,
        "largest_group": largest,
        "max_candidates": login.max_candidates,
        "match_policy": login.match_policy,
        "exceeds_max_candidates": login.voters_per_value > login.max_candidates,
    })
}

/// Generate bounded CSV files without allocating one object per voter.
/// Keycloak's PBKDF2-SHA256 credential format fixes the salt and digest sizes.
/// A plaintext password column must never accompany the supplied hash.
pub fn generate(input: &Input, output: &Path) -> Result<()> {
    input.validate()?;
    let election_id = input
        .event
        .election_external_id
        .as_deref()
        .filter(|id| !id.is_empty())
        .unwrap_or(&input.event.election_id);
    ensure!(
        !election_id.contains(MULTIVALUE_USER_ATTRIBUTE_SEPARATOR),
        "Election authorization ID cannot contain the census multivalue separator '|'"
    );
    let workload = &input.settings.workload;
    let login = &workload.login;
    files::claim_directory(output)?;
    let start = Instant::now();
    let mut salt = [0; 16];
    SystemRandom::new()
        .fill(&mut salt)
        .map_err(|_| anyhow::anyhow!("Cannot generate census salt"))?;
    let rounds =
        NonZeroU32::new(workload.hash_iterations).context("Hash rounds must be positive")?;
    let password = input.password()?;
    let suffixes: Vec<String> = (0..login.voters_per_value as u64)
        .map(|position| password_suffix(login, position))
        .collect();
    let hashes: Vec<String> = suffixes
        .iter()
        .map(|suffix| {
            let mut digest = [0; 32];
            pbkdf2::derive(
                pbkdf2::PBKDF2_HMAC_SHA256,
                rounds,
                &salt,
                format!("{password}{suffix}").as_bytes(),
                &mut digest,
            );
            STANDARD.encode(digest)
        })
        .collect();
    let salt = STANDARD.encode(salt);
    let rounds = rounds.to_string();
    for shard in 0..input.shards() {
        let (first, count) = input.bounds(shard)?;
        let mut csv =
            csv::Writer::from_writer(files::create(&output.join(format!("{shard:06}.csv")))?);
        csv.write_record(
            CENSUS_COLUMNS
                .iter()
                .copied()
                .chain(login.match_attributes.iter().map(String::as_str)),
        )?;
        for index in first..first + count as u64 {
            let username = format!("{}{index}", workload.username_prefix);
            let email = format!("{username}@example.invalid");
            let attributes = login
                .match_attributes
                .iter()
                .map(|attribute| attribute_value(attribute, group(login, index)))
                .collect::<Result<Vec<_>>>()?;
            let fixed = [
                username.as_str(),
                &input.event.area_name,
                &email,
                "true",
                election_id,
                &hashes[position(login, index) as usize],
                &salt,
                &rounds,
            ];
            csv.write_record(
                fixed
                    .into_iter()
                    .chain(attributes.iter().map(String::as_str)),
            )?;
        }
        csv.flush()?;
    }
    files::save(
        &output.join("census.json"),
        &json!({"count":workload.count,
        "shards":input.shards(), "password_hash_computations":hashes.len(), "elapsed_seconds":start.elapsed().as_secs_f64(),
        "tenant_id":input.settings.target.tenant_id, "election_event_id":input.event.election_event_id,
        "login": distribution(login, workload.start, workload.count)}),
    )
}
