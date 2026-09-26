// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Streaming census generation: one shared PBKDF2 hash, unique exact usernames.
use super::{files, input::Input};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use ring::{
    pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
use serde_json::json;
use std::{num::NonZeroU32, path::Path, time::Instant};

/// Generate bounded CSV files without allocating one object per voter.
/// Keycloak's PBKDF2-SHA256 credential format fixes the salt and digest sizes.
/// A plaintext password column must never accompany the supplied hash.
pub fn generate(input: &Input, output: &Path) -> Result<()> {
    input.validate()?;
    files::claim_directory(output)?;
    let start = Instant::now();
    let mut salt = [0; 16];
    SystemRandom::new()
        .fill(&mut salt)
        .map_err(|_| anyhow::anyhow!("Cannot generate census salt"))?;
    let mut digest = [0; 32];
    let rounds = NonZeroU32::new(input.settings.workload.hash_iterations)
        .context("Hash rounds must be positive")?;
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        rounds,
        &salt,
        input.password()?.as_bytes(),
        &mut digest,
    );
    let hash = STANDARD.encode(digest);
    let salt = STANDARD.encode(salt);
    let rounds = rounds.to_string();
    for shard in 0..input.shards() {
        let (first, count) = input.bounds(shard)?;
        let mut csv =
            csv::Writer::from_writer(files::create(&output.join(format!("{shard:06}.csv")))?);
        csv.write_record([
            "username",
            "area_name",
            "email",
            "email_verified",
            "authorized-election-ids",
            "hashed_password",
            "password_salt",
            "num_of_iterations",
        ])?;
        for index in first..first + count as u64 {
            let username = format!("{}{index}", input.settings.workload.username_prefix);
            csv.write_record([
                &username,
                &input.event.area_name,
                &format!("{username}@example.invalid"),
                "true",
                &input.event.election_id,
                &hash,
                &salt,
                &rounds,
            ])?;
        }
        csv.flush()?;
    }
    files::save(
        &output.join("census.json"),
        &json!({"count":input.settings.workload.count,
        "shards":input.shards(), "password_hash_computations":1, "elapsed_seconds":start.elapsed().as_secs_f64(),
        "tenant_id":input.settings.target.tenant_id, "election_event_id":input.event.election_event_id}),
    )
}
