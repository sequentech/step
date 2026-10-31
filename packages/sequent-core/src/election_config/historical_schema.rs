// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The two source schemas `legacy-seat-v1` reads.
//!
//! The legacy dump (`ovcs_ver7`) names its tables in the singular. The documented
//! post-qualification dump (`ovcs_post_qua`) names most of them in the plural,
//! renames a few columns and adds `trans_servers`. Both describe the same records,
//! so the post-qualification names are mapped onto the legacy ones once, here, and
//! the conversion reads a single shape. Problems are written back in the source's
//! own names, so a refusal points at the table and column the dump actually has.
//!
//! Only names are mapped. A column whose meaning changed is not renamed: the
//! voters' mail location (`MAILCOUNTRY`, `MAILEMBASSY`) is read by its own voter
//! area join rather than passed off as the legacy `COUNTRY`/`EMBASSY`.

use super::super::{
    problem::{Code, Problem, Report},
    sql_dump::{SqlDump, SqlTable},
};
use serde::Serialize;

/// Which schema the dump was written in, recorded in the summary.
#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SourceSchema {
    #[default]
    Legacy,
    PostQualification,
}

/// `(post-qualification table, legacy table)`.
const TABLES: &[(&str, &str)] = &[
    ("candidates", "candidate"),
    ("contests", "contest"),
    ("seats", "seat"),
    ("political_party", "party_list"),
    ("canvass_centers", "canvass_center"),
    ("trans_routes", "trans_route"),
    ("jurisdictions", "geographical"),
];

/// `(legacy table, post-qualification column, legacy column)`: the same value
/// under a new name.
const COLUMNS: &[(&str, &str, &str)] = &[
    ("seat", "NUM_SEAT", "ELIGIBLEAMOUNT"),
    ("seat", "PROC_CCS_CODE", "CCS_CODE"),
    ("pop", "VOTING_MODE", "MACHINE_TYPE"),
    ("trans_route", "TYPE", "MACHINE_TYPE"),
];

/// Tables a dump may carry that the conversion does not read.
pub const OPTIONAL_TABLES: &[&str] = &["trans_servers"];

/// Recognise the dump's schema and map a post-qualification dump onto the legacy
/// names. A dump that mixes both sets of names is refused rather than guessed at.
pub fn normalize(
    mut dump: SqlDump,
) -> Result<(SqlDump, SourceSchema), Problem> {
    let has = |name: &str| dump.tables.contains_key(name);
    let post = TABLES.iter().filter(|(source, _)| has(source)).count();
    let legacy = TABLES
        .iter()
        .filter(|(_, canonical)| has(canonical))
        .count();
    if post == 0 {
        return Ok((dump, SourceSchema::Legacy));
    }
    if legacy > 0 {
        return Err(Problem::error(
            Code::InvalidValue,
            "SQL schema",
            "the dump mixes legacy and post-qualification table names",
        ));
    }
    for (source, canonical) in TABLES {
        if let Some(table) = dump.tables.remove(*source) {
            dump.tables.insert((*canonical).into(), table);
        }
    }
    for (table, source, canonical) in COLUMNS {
        if let Some(table) = dump.tables.get_mut(*table) {
            rename_column(table, source, canonical);
        }
    }
    Ok((dump, SourceSchema::PostQualification))
}

fn rename_column(table: &mut SqlTable, from: &str, to: &str) {
    let rename = |column: &mut String| {
        if column == from {
            *column = to.into();
        }
    };
    table.columns.iter_mut().for_each(rename);
    table.primary_key.iter_mut().for_each(rename);
    table
        .unique_keys
        .iter_mut()
        .flat_map(|key| key.iter_mut())
        .for_each(rename);
    if let Some(charset) = table.column_charsets.remove(from) {
        table.column_charsets.insert(to.into(), charset);
    }
}

/// Write a report's SQL locations in the source's own table and column names.
pub fn source_names(mut report: Report, schema: SourceSchema) -> Report {
    if schema == SourceSchema::PostQualification {
        for problem in &mut report.problems {
            problem.path = source_path(&problem.path);
        }
    }
    report
}

/// `SQL table seat row 3 column ELIGIBLEAMOUNT` →
/// `SQL table seats row 3 column NUM_SEAT`.
fn source_path(path: &str) -> String {
    const PREFIX: &str = "SQL table ";
    let Some(rest) = path.strip_prefix(PREFIX) else {
        return path.into();
    };
    let (table, tail) = rest.split_once(' ').unwrap_or((rest, ""));
    let source = TABLES
        .iter()
        .find(|(_, canonical)| *canonical == table)
        .map_or(table, |(source, _)| source);
    let tail = COLUMNS.iter().filter(|(t, _, _)| *t == table).fold(
        tail.to_string(),
        |tail, (_, source, canonical)| match tail
            .strip_suffix(&format!("column {canonical}"))
        {
            Some(head) => format!("{head}column {source}"),
            None => tail,
        },
    );
    if tail.is_empty() {
        format!("{PREFIX}{source}")
    } else {
        format!("{PREFIX}{source} {tail}")
    }
}
