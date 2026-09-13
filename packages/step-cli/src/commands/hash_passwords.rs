// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use clap::Args;
use colored::Colorize;
use csv::{ReaderBuilder, StringRecord, WriterBuilder};
use rand::thread_rng;
use rand::Rng;
use rayon::prelude::*;
use ring::{digest, pbkdf2};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::num::NonZeroU32;
use std::path::Path;
use tempfile::NamedTempFile;

const CREDENTIAL_LEN: usize = digest::SHA256_OUTPUT_LEN;
const PASSWORD_COLUMN: &str = "password";
const PASSWORD_BATCH_SIZE: usize = 256;
const CREDENTIAL_COLUMNS: [&str; 3] = ["password_salt", "hashed_password", "num_of_iterations"];
pub type Credential = [u8; CREDENTIAL_LEN];
static PBKDF2_ALGORITHM: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA256;

#[derive(Args)]
#[command(about = "Process a CSV file to hash passwords and generate salts")]
pub struct HashPasswords {
    #[arg(long)]
    input_file: String,

    #[arg(long)]
    output_file: String,

    #[arg(long, default_value_t = NonZeroU32::new(600000).unwrap())]
    iterations: NonZeroU32,
}

impl HashPasswords {
    pub fn run(&self) -> Result<()> {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(self.run_hash_password())?;
        println!("{}", "Successfully generated hashed passwords".green());
        Ok(())
    }

    /// Convert a complete CSV before replacing the destination. Invalid input
    /// leaves an earlier export intact, and temporary credentials stay private.
    pub async fn run_hash_password(&self) -> Result<()> {
        let input_path = Path::new(&self.input_file).canonicalize()?;
        let output_path = Path::new(&self.output_file);
        if output_path.canonicalize().ok().as_ref() == Some(&input_path) {
            bail!("Input and output must be different files");
        }
        let input = File::open(&self.input_file)?;
        let mut rdr = ReaderBuilder::new().from_reader(BufReader::new(input));

        let original_headers = rdr.headers()?.clone();
        if original_headers
            .iter()
            .filter(|name| *name == PASSWORD_COLUMN)
            .count()
            != 1
        {
            bail!("CSV must contain exactly one 'password' column");
        }
        if original_headers
            .iter()
            .any(|name| CREDENTIAL_COLUMNS.contains(&name))
        {
            bail!("CSV already contains credential output columns");
        }
        let password_index = original_headers
            .iter()
            .position(|name| name == PASSWORD_COLUMN)
            .expect("exactly one password header was validated above");

        let mut new_headers = StringRecord::new();
        for (i, header) in original_headers.iter().enumerate() {
            if i != password_index {
                new_headers.push_field(header);
            }
        }
        for name in CREDENTIAL_COLUMNS {
            new_headers.push_field(name);
        }

        // A sibling temporary file keeps the final rename on one filesystem.
        // NamedTempFile uses private permissions and removes unfinished output
        // on error; persisting it replaces the destination only after success.
        let parent = output_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut temporary = NamedTempFile::new_in(parent)?;
        {
            let mut wtr = WriterBuilder::new().from_writer(BufWriter::new(temporary.as_file_mut()));
            wtr.write_record(&new_headers)?;
            write_hashed_records(rdr.records(), &mut wtr, password_index, self.iterations)?;
            wtr.flush()?;
        }
        temporary.as_file().sync_all()?;
        temporary
            .persist(output_path)
            .context("Unable to replace credential export")?;
        Ok(())
    }
}

/// Bound plaintext retention and parallel hashing to one batch. The caller owns
/// the temporary file and publishes it only after this entire iterator succeeds.
fn write_hashed_records<W: std::io::Write>(
    mut records: impl Iterator<Item = csv::Result<StringRecord>>,
    writer: &mut csv::Writer<W>,
    password_index: usize,
    iterations: NonZeroU32,
) -> Result<()> {
    loop {
        let batch = records
            .by_ref()
            .take(PASSWORD_BATCH_SIZE)
            .collect::<Result<Vec<_>, _>>()?;
        if batch.is_empty() {
            return Ok(());
        }
        // Indexed parallel iteration preserves source order. Consuming each
        // record also drops its plaintext when that record has been converted.
        let processed: Vec<StringRecord> = batch
            .into_par_iter()
            .map(|record| hash_record(&record, password_index, iterations))
            .collect();
        for record in processed {
            writer.write_record(&record)?;
        }
    }
}

fn hash_record(
    record: &StringRecord,
    password_index: usize,
    iterations: NonZeroU32,
) -> StringRecord {
    let password = record.get(password_index).unwrap_or("");
    let mut salt: Credential = Default::default();
    thread_rng().fill(&mut salt);
    let derived = hash_password(password, &salt, iterations);
    let mut output = StringRecord::new();
    for (index, field) in record.iter().enumerate() {
        if index != password_index {
            output.push_field(field);
        }
    }
    output.push_field(&BASE64_STANDARD.encode(salt));
    output.push_field(&derived);
    output.push_field(&iterations.to_string());
    output
}

fn hash_password(password: &str, salt: &[u8], iterations: NonZeroU32) -> String {
    let mut output: Credential = [0u8; CREDENTIAL_LEN];
    pbkdf2::derive(
        PBKDF2_ALGORITHM,
        iterations,
        salt,
        password.as_bytes(),
        &mut output,
    );
    BASE64_STANDARD.encode(output)
}

#[cfg(test)]
#[path = "../../tests/support/password_boundaries.rs"]
mod boundary_tests;
