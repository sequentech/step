// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
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
use tracing::{error, info};

/// Length of the credential hash
const CREDENTIAL_LEN: usize = digest::SHA256_OUTPUT_LEN;
/// Type of the credential hash
pub type Credential = [u8; CREDENTIAL_LEN];
/// PBKDF2 algorithm to use
static PBKDF2_ALGORITHM: pbkdf2::Algorithm = pbkdf2::PBKDF2_HMAC_SHA256;

#[derive(Args)]
#[command(about = "Process a CSV file to hash passwords and generate salts")]
/// Command to hash passwords and generate salts
pub struct HashPasswords {
    /// Input file to hash passwords from
    #[arg(long)]
    input_file: String,

    /// Output file to save the results to
    #[arg(long)]
    output_file: String,

    /// Number of iterations to use for the PBKDF2 algorithm
    #[arg(long, default_value_t = NonZeroU32::new(600000).unwrap())]
    iterations: NonZeroU32,
}

impl HashPasswords {
    /// Run the command
    pub fn run(&self) {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        match self.run_hash_password() {
            Ok(()) => info!("Successfully generated hashed passwords"),
            Err(err) => error!("Error! Failed to generate hashed password: {err:?}"),
        }
    }

    ///Hash password in the input file and save the results to the output file
    pub fn run_hash_password(&self) -> Result<()> {
        let input = File::open(&self.input_file)?;
        let mut rdr = ReaderBuilder::new().from_reader(BufReader::new(input));

        let output = File::create(&self.output_file)?;
        let mut wtr = WriterBuilder::new().from_writer(BufWriter::new(output));

        let original_headers = rdr.headers()?.clone();
        let password_index = original_headers
            .iter()
            .position(|h| h == "password")
            .ok_or_else(|| anyhow::anyhow!("CSV does not contain a 'password' column"))?;

        let mut new_headers = StringRecord::new();
        for (i, header) in original_headers.iter().enumerate() {
            if i != password_index {
                new_headers.push_field(header);
            }
        }
        new_headers.push_field("password_salt");
        new_headers.push_field("hashed_password");
        new_headers.push_field("num_of_iterations");

        wtr.write_record(&new_headers)?;

        let records: Vec<StringRecord> = rdr.records().collect::<Result<Vec<_>, _>>()?;

        let processed_records: Vec<anyhow::Result<StringRecord>> = records
            .par_iter()
            .map(|record| {
                let password = record.get(password_index).unwrap_or("");
                let mut salt_bytes: Credential = Default::default();
                thread_rng().fill(&mut salt_bytes);
                let password_salt = BASE64_STANDARD.encode(salt_bytes);
                let hashed_password = hash_password(
                    &password.to_string(),
                    salt_bytes.as_slice(),
                    self.iterations,
                );
                let new_fields: Vec<&str> = record
                    .iter()
                    .enumerate()
                    .filter_map(|(i, field)| {
                        if i == password_index {
                            None
                        } else {
                            Some(field)
                        }
                    })
                    .collect();
                let mut new_record = StringRecord::from(new_fields);
                new_record.push_field(&password_salt);
                new_record.push_field(&hashed_password);
                new_record.push_field(&self.iterations.to_string());
                Ok(new_record)
            })
            .collect();

        for record in processed_records {
            let record = record?;
            wtr.write_record(&record)?;
        }

        wtr.flush()?;
        Ok(())
    }
}

/// Hash password
fn hash_password(password: &String, salt: &[u8], iterations: NonZeroU32) -> String {
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
