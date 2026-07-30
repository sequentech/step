// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Parses the reconciliation file format, a `#META` comment line, a CSV header,
//! then one row per voter. Modeled on
//! `services::tally_sheet_import::csv::parse_canonical_csv` — per-row errors
//! are collected, not fatal, since a handful of malformed rows shouldn't sink
//! a 100k-row file; the caller decides whether accumulated errors should
//! still block the whole import.

use crate::services::external::datafix_types::ParsedDatafixReconciliationRow;
use crate::services::external::types::ReconciliationFileMeta;
use ::csv::{ReaderBuilder, StringRecord};
use sequent_core::types::keycloak::ATTR_RESET_VALUE;
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use tracing::instrument;

const RECONCILIATION_HEADERS: [&str; 8] = [
    "CountyMun",
    "VoterID",
    "DoB",
    "Ward",
    "Poll",
    "SchoolSupportCode",
    "Channel",
    "Deleted",
];

/// A row that failed to parse, kept by (1-indexed, header-exclusive) line
/// number so the operator can find it in the original file. Line zero
/// denotes an invalid or missing CSV header.
#[derive(Debug, Clone)]
pub struct RowParseError {
    pub line: usize,
    pub message: String,
}

/// Parses the mandatory `#META,Sequence=N,GeneratedAt=T` line. Sequence zero
/// is a legitimate kickoff value, so malformed or missing metadata must be
/// rejected rather than silently mapped to zero.
#[instrument(skip_all)]
pub fn parse_meta_line(line: &str) -> Result<ReconciliationFileMeta, String> {
    let line = line.trim_end_matches('\r');
    let fields = line
        .strip_prefix("#META,")
        .ok_or_else(|| "the first line must start with '#META,'".to_string())?;
    let mut sequence = None;
    let mut generated_at = None;
    let mut seen = HashSet::new();
    for field in fields.split(',') {
        let (key, value) = field
            .split_once('=')
            .ok_or_else(|| format!("invalid metadata field '{field}'"))?;
        if !seen.insert(key) {
            return Err(format!("duplicate metadata field '{key}'"));
        }
        match key {
            "Sequence" => {
                let parsed = value
                    .parse::<i64>()
                    .map_err(|_| format!("Sequence '{value}' is not an integer"))?;
                if parsed < 0 {
                    return Err("Sequence cannot be negative".to_string());
                }
                sequence = Some(parsed);
            }
            "GeneratedAt" => {
                let parsed = value
                    .parse::<i64>()
                    .map_err(|_| format!("GeneratedAt '{value}' is not an integer"))?;
                if parsed < 0 {
                    return Err("GeneratedAt cannot be negative".to_string());
                }
                generated_at = Some(parsed);
            }
            _ => return Err(format!("unknown metadata field '{key}'")),
        }
    }
    Ok(ReconciliationFileMeta {
        sequence: sequence.ok_or_else(|| "metadata is missing Sequence".to_string())?,
        generated_at: generated_at.ok_or_else(|| "metadata is missing GeneratedAt".to_string())?,
    })
}

/// Splits the raw file bytes into the `#META` line and the remaining CSV
/// (header + rows), so callers don't need to know the file starts with a
/// non-CSV comment line before handing the rest to a `csv::Reader`.
#[instrument(skip(bytes))]
pub fn split_meta_and_csv(bytes: &[u8]) -> Result<(ReconciliationFileMeta, &[u8]), String> {
    let text_len = bytes.len();
    let newline_pos = bytes.iter().position(|&byte| byte == b'\n');
    let (meta_line_bytes, rest) = match newline_pos {
        Some(pos) => (&bytes[..pos], &bytes[(pos + 1).min(text_len)..]),
        None => return Err("reconciliation file has no CSV body after #META".to_string()),
    };
    let meta_line = std::str::from_utf8(meta_line_bytes)
        .map_err(|_| "#META line is not valid UTF-8".to_string())?;
    Ok((parse_meta_line(meta_line)?, rest))
}

/// Datafix represents unset optional area components both as an empty CSV
/// cell and as `NONE`. Normalize the former at the input boundary so every
/// downstream comparison and generated patch uses the canonical sentinel.
fn normalize_optional_area_fields(row: &mut ParsedDatafixReconciliationRow) {
    for value in [&mut row.poll, &mut row.school_support_code] {
        if value.is_empty() {
            *value = ATTR_RESET_VALUE.to_string();
        }
    }
}

fn validate_row(row: &ParsedDatafixReconciliationRow) -> Result<(), String> {
    let values = [
        ("CountyMun", row.county_mun.as_str()),
        ("VoterID", row.external_voter_id.as_str()),
        ("DoB", row.dob.as_str()),
        ("Ward", row.ward.as_str()),
        ("Poll", row.poll.as_str()),
        ("SchoolSupportCode", row.school_support_code.as_str()),
        ("Channel", row.channel.as_str()),
        ("Deleted", row.deleted.as_str()),
    ];
    for (name, value) in values {
        if value.is_empty() {
            return Err(format!("{name} cannot be empty; use NONE when unset"));
        }
        if value.trim() != value {
            return Err(format!("{name} cannot have leading or trailing whitespace"));
        }
    }
    if row.channel != row.channel.to_uppercase() {
        return Err("Channel must be uppercase".to_string());
    }
    if !matches!(row.deleted.as_str(), "true" | "false") {
        return Err("Deleted must be exactly 'true' or 'false'".to_string());
    }
    Ok(())
}

fn validate_headers(headers: &StringRecord) -> Result<(), String> {
    if headers.iter().eq(RECONCILIATION_HEADERS) {
        Ok(())
    } else {
        Err(format!(
            "CSV header must be exactly: {}",
            RECONCILIATION_HEADERS.join(",")
        ))
    }
}

/// Incrementally reads reconciliation rows in fixed-size batches, the
/// production parser, so a 100k+-row file's rows are never all resident in
/// memory at once, only whichever batch is currently being processed. Stops
/// at the first malformed row
/// rather than collecting every error across the whole file: nothing this
/// pipeline does is applied to voter data until a later, separate step, so
/// discovering a bad row late just wastes the processing done so far, it
/// doesn't risk anything being half-applied.
pub struct ReconciliationRowBatches<R: Read> {
    reader: ::csv::Reader<R>,
    next_line: usize,
    headers_validated: bool,
}

impl<R: Read> ReconciliationRowBatches<R> {
    /// `body` must already be positioned past the `#META` line, at the CSV
    /// header.
    pub fn new(body: R) -> Self {
        Self {
            reader: ReaderBuilder::new().has_headers(true).from_reader(body),
            next_line: 1,
            headers_validated: false,
        }
    }

    /// Reads up to `batch_size` rows. An empty result (with no error) means
    /// the file is exhausted.
    pub fn next_batch(
        &mut self,
        batch_size: usize,
    ) -> std::result::Result<Vec<ParsedDatafixReconciliationRow>, RowParseError> {
        if !self.headers_validated {
            let headers = self.reader.headers().map_err(|err| RowParseError {
                line: 0,
                message: err.to_string(),
            })?;
            validate_headers(headers).map_err(|message| RowParseError { line: 0, message })?;
            self.headers_validated = true;
        }
        let mut rows = Vec::with_capacity(batch_size);
        let mut records = self.reader.deserialize::<ParsedDatafixReconciliationRow>();
        for _ in 0..batch_size {
            match records.next() {
                Some(Ok(mut row)) => {
                    normalize_optional_area_fields(&mut row);
                    match validate_row(&row) {
                        Ok(()) => {
                            rows.push(row);
                            self.next_line += 1;
                        }
                        Err(message) => {
                            return Err(RowParseError {
                                line: self.next_line,
                                message,
                            });
                        }
                    }
                }
                Some(Err(err)) => {
                    return Err(RowParseError {
                        line: self.next_line,
                        message: err.to_string(),
                    });
                }
                None => break,
            }
        }
        Ok(rows)
    }
}

impl ReconciliationRowBatches<BufReader<std::fs::File>> {
    /// Opens `path` and skips exactly one line — the `#META` line, already
    /// parsed and validated separately by the caller (via `parse_meta_line`
    /// against the same file's bytes) before this is ever called — to
    /// position the reader at the CSV header.
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut discarded_meta_line = String::new();
        reader.read_line(&mut discarded_meta_line)?;
        Ok(Self::new(reader))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_all_streamed(
        csv_bytes: &[u8],
    ) -> std::result::Result<Vec<ParsedDatafixReconciliationRow>, RowParseError> {
        let mut reader = ReconciliationRowBatches::new(csv_bytes);
        let mut rows = Vec::new();
        loop {
            let batch = reader.next_batch(2)?;
            if batch.is_empty() {
                return Ok(rows);
            }
            rows.extend(batch);
        }
    }

    #[test]
    fn parses_sequence_and_generated_at() {
        let meta = parse_meta_line("#META,Sequence=42,GeneratedAt=1781780700").unwrap();
        assert_eq!(meta.sequence, 42);
        assert_eq!(meta.generated_at, 1781780700);
    }

    #[test]
    fn rejects_missing_meta_line_and_fields() {
        assert!(parse_meta_line("CountyMun,VoterID").is_err());
        assert!(parse_meta_line("#META,GeneratedAt=5").is_err());
        assert!(parse_meta_line("#META,Sequence=0").is_err());
    }

    #[test]
    fn rejects_unparseable_or_negative_metadata() {
        assert!(parse_meta_line("#META,Sequence=not-a-number,GeneratedAt=5").is_err());
        assert!(parse_meta_line("#META,Sequence=-1,GeneratedAt=5").is_err());
        assert!(parse_meta_line("#META,Sequence=1,GeneratedAt=tomorrow").is_err());
    }

    #[test]
    fn splits_meta_line_from_csv_body() {
        let file = b"#META,Sequence=1,GeneratedAt=2\nCountyMun,VoterID,DoB,Ward,Poll,SchoolSupportCode,Channel,Deleted\n0014,17695,1963-05-23,04,000,P,NONE,false\n";
        let (meta, csv_bytes) = split_meta_and_csv(file).unwrap();
        assert_eq!(meta.sequence, 1);
        let rows = parse_all_streamed(csv_bytes).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].external_voter_id, "17695");
        assert_eq!(rows[0].channel, "NONE");
        assert_eq!(rows[0].deleted, "false");
    }

    #[test]
    fn rejects_a_plain_csv_without_meta_instead_of_swallowing_its_header() {
        let file = b"CountyMun,VoterID,DoB,Ward,Poll,SchoolSupportCode,Channel,Deleted\n0014,17695,1963-05-23,04,000,P,NONE,false\n";
        let error = split_meta_and_csv(file).unwrap_err();
        assert!(error.contains("#META"));
    }

    #[test]
    fn rejects_a_missing_or_reordered_header_even_when_there_are_no_rows() {
        for csv_bytes in [
            b"".as_slice(),
            b"VoterID,CountyMun,DoB,Ward,Poll,SchoolSupportCode,Channel,Deleted\n".as_slice(),
        ] {
            let error = parse_all_streamed(csv_bytes).unwrap_err();
            assert_eq!(error.line, 0);
        }
    }

    #[test]
    fn rejects_non_uppercase_channels_and_noncanonical_booleans() {
        for (row, expected) in [
            (
                "0014,17695,1963-05-23,04,000,P,Internet,false",
                "Channel must be uppercase",
            ),
            (
                "0014,17696,1963-05-23,04,000,P,NONE,FALSE",
                "Deleted must be exactly",
            ),
        ] {
            let csv = format!(
                "CountyMun,VoterID,DoB,Ward,Poll,SchoolSupportCode,Channel,Deleted\n{row}\n"
            );
            let error = parse_all_streamed(csv.as_bytes()).unwrap_err();
            assert!(error.message.contains(expected));
        }
    }

    #[test]
    fn normalizes_empty_optional_area_fields_to_none() {
        let csv_bytes = b"CountyMun,VoterID,DoB,Ward,Poll,SchoolSupportCode,Channel,Deleted\n0014,17695,1963-05-23,04,,P,NONE,false\n0014,17696,1963-05-23,04,000,,NONE,false\n0014,17697,1963-05-23,04,,,NONE,false\n";
        let rows = parse_all_streamed(csv_bytes).unwrap();

        assert_eq!(rows[0].poll, ATTR_RESET_VALUE);
        assert_eq!(rows[0].school_support_code, "P");
        assert_eq!(rows[1].poll, "000");
        assert_eq!(rows[1].school_support_code, ATTR_RESET_VALUE);
        assert_eq!(rows[2].poll, ATTR_RESET_VALUE);
        assert_eq!(rows[2].school_support_code, ATTR_RESET_VALUE);
    }

    #[test]
    fn still_rejects_empty_required_fields() {
        let csv_bytes = b"CountyMun,VoterID,DoB,Ward,Poll,SchoolSupportCode,Channel,Deleted\n0014,,1963-05-23,04,000,P,NONE,false\n";
        let error = parse_all_streamed(csv_bytes).unwrap_err();
        assert!(error.message.contains("VoterID cannot be empty"));
    }

    #[test]
    fn streaming_reader_stops_at_the_first_malformed_row() {
        let csv_bytes = b"CountyMun,VoterID,DoB,Ward,Poll,SchoolSupportCode,Channel,Deleted\n0014,17695,1963-05-23,04,000,P,NONE,false\n0014,79535,1948-04-06\n0014,68684,1978-02-28,03,000,P,NONE,false\n";
        let mut reader = ReconciliationRowBatches::new(csv_bytes.as_slice());
        assert_eq!(reader.next_batch(1).unwrap().len(), 1);
        let error = reader.next_batch(1).unwrap_err();
        assert_eq!(error.line, 2);
    }
}
