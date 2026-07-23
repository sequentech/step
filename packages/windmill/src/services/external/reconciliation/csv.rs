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
use ::csv::ReaderBuilder;
use tracing::instrument;

/// A row that failed to parse, kept by (1-indexed, header-exclusive) line
/// number so the operator can find it in the original file.
#[derive(Debug, Clone)]
pub struct RowParseError {
    pub line: usize,
    pub message: String,
}

/// Parses the `#META,Sequence=N,GeneratedAt=T` line. Missing/unparseable
/// fields default to `0` rather than failing the whole file — the caller
/// rejects on the `Sequence` check downstream, which gives a clearer error
/// than a raw parse failure would.
#[instrument(skip_all)]
pub fn parse_meta_line(line: &str) -> ReconciliationFileMeta {
    let mut sequence = 0i64;
    let mut generated_at = 0i64;
    for field in line
        .trim()
        .trim_start_matches("#META")
        .trim_start_matches(',')
        .split(',')
    {
        if let Some((key, value)) = field.split_once('=') {
            match key {
                "Sequence" => sequence = value.parse().unwrap_or(0),
                "GeneratedAt" => generated_at = value.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    ReconciliationFileMeta {
        sequence,
        generated_at,
    }
}

/// Splits the raw file bytes into the `#META` line and the remaining CSV
/// (header + rows), so callers don't need to know the file starts with a
/// non-CSV comment line before handing the rest to a `csv::Reader`.
#[instrument(skip(bytes))]
pub fn split_meta_and_csv(bytes: &[u8]) -> (ReconciliationFileMeta, &[u8]) {
    let text_len = bytes.len();
    let newline_pos = bytes.iter().position(|&byte| byte == b'\n');
    let (meta_line_bytes, rest) = match newline_pos {
        Some(pos) => (&bytes[..pos], &bytes[(pos + 1).min(text_len)..]),
        None => (bytes, &bytes[text_len..]),
    };
    let meta_line = String::from_utf8_lossy(meta_line_bytes);
    (parse_meta_line(&meta_line), rest)
}

/// Parses the reconciliation CSV body (everything after the `#META` line) into
/// typed rows, collecting per-row errors instead of aborting on the first bad
/// row — mirrors `tally_sheet_import::csv::parse_canonical_csv`.
#[instrument(skip(csv_bytes))]
pub fn parse_reconciliation_rows(
    csv_bytes: &[u8],
) -> (Vec<ParsedDatafixReconciliationRow>, Vec<RowParseError>) {
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(csv_bytes);
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    for (index, record) in reader
        .deserialize::<ParsedDatafixReconciliationRow>()
        .enumerate()
    {
        match record {
            Ok(row) => rows.push(row),
            Err(err) => errors.push(RowParseError {
                line: index + 1,
                message: err.to_string(),
            }),
        }
    }
    (rows, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sequence_and_generated_at() {
        let meta = parse_meta_line("#META,Sequence=42,GeneratedAt=1781780700");
        assert_eq!(meta.sequence, 42);
        assert_eq!(meta.generated_at, 1781780700);
    }

    #[test]
    fn defaults_missing_fields_to_zero() {
        let meta = parse_meta_line("#META");
        assert_eq!(meta.sequence, 0);
        assert_eq!(meta.generated_at, 0);
    }

    #[test]
    fn defaults_unparseable_sequence_to_zero() {
        let meta = parse_meta_line("#META,Sequence=not-a-number,GeneratedAt=5");
        assert_eq!(meta.sequence, 0);
        assert_eq!(meta.generated_at, 5);
    }

    #[test]
    fn splits_meta_line_from_csv_body() {
        let file = b"#META,Sequence=1,GeneratedAt=2\nCountyMun,VoterID,DoB,Ward,Poll,SchoolSupportCode,Channel,Deleted\n0014,17695,1963-05-23,04,000,P,NONE,false\n";
        let (meta, csv_bytes) = split_meta_and_csv(file);
        assert_eq!(meta.sequence, 1);
        let (rows, errors) = parse_reconciliation_rows(csv_bytes);
        assert!(errors.is_empty());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].voter_id, "17695");
        assert_eq!(rows[0].channel, "NONE");
        assert_eq!(rows[0].deleted, "false");
    }

    #[test]
    fn collects_malformed_rows_without_aborting_the_whole_file() {
        // Second row is missing a column - csv::Reader flags a field-count
        // mismatch as an error for that row only.
        let csv_bytes = b"CountyMun,VoterID,DoB,Ward,Poll,SchoolSupportCode,Channel,Deleted\n0014,17695,1963-05-23,04,000,P,NONE,false\n0014,79535,1948-04-06\n0014,68684,1978-02-28,03,000,P,NONE,false\n";
        let (rows, errors) = parse_reconciliation_rows(csv_bytes);
        assert_eq!(rows.len(), 2);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 2);
    }
}
