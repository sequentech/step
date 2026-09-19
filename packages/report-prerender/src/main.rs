// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! File-based native adapter for Studio and offline batch jobs. JSONL records
//! are consumed one at a time; no runtime browser or network service is used.
use anyhow::{bail, Context, Result};
use sequent_report_prerender::{fill_many, validate_background, Prepared, DEFAULT_FONT};
use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Read},
};
fn read(path: &str, max: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(max + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        bail!("File exceeds input limit");
    }
    Ok(bytes)
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let validate = args.len() == 4 && args[1] == "validate";
    if !validate && !(args.len() == 6 && args[1] == "fill") {
        bail!("Usage: sequent-report-prerender validate background.pdf manifest.json\n       sequent-report-prerender fill background.pdf manifest.json records.jsonl output.pdf");
    }
    let background = read(&args[2], 24_000_000)?;
    let prepared: Prepared = serde_json::from_slice(&read(&args[3], 8_000_000)?)?;
    if validate {
        return validate_background(&background, &prepared);
    }
    let mut input = BufReader::new(File::open(&args[4])?);
    let records = std::iter::from_fn(move || {
        let mut line = Vec::new();
        match input.by_ref().take(8_000_001).read_until(b'\n', &mut line) {
            Ok(0) => None,
            Ok(n) if n > 8_000_000 => Some(Err(anyhow::anyhow!("Runtime JSONL line exceeds 8 MB"))),
            Ok(_) => Some(serde_json::from_slice(&line).context("Invalid JSONL record")),
            Err(e) => Some(Err(e.into())),
        }
    });
    let output = File::options()
        .create_new(true)
        .write(true)
        .open(&args[5])?;
    let result = fill_many(
        &background,
        &prepared,
        DEFAULT_FONT,
        records,
        BufWriter::new(output),
    );
    if result.is_err() {
        let _ = std::fs::remove_file(&args[5]);
    }
    result?;
    Ok(())
}
