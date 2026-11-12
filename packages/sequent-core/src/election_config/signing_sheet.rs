// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! An event's signing configuration as workbook sheets, and as a plan field.
//!
//! The bundle carries `signing_rules` and `signing_checks` (see
//! [`super::ImportElectionEventSchema`]), and the platform's importer saves
//! both on the new event. Until these sheets existed only a base export could
//! supply them, so a configuration opened in the wizard, written to its
//! workbook or converted from a historical source lost them without a word.
//!
//! Two sheets, each **optional and independent**:
//!
//! - `SigningRules`: one row per action: `action`, `requirement`,
//!   `signatures`, `requester_signing`, `expires_minutes`, `revision`.
//! - `SigningChecks`: one row: `revocation_check`, `crl_unavailable`,
//!   `registration`, `post_binding`, `revision`.
//! - `ApprovalMatrix`: one row, one `matrix` cell holding the bundle's
//!   `approval_matrix` as JSON (no screen edits it; it is carried as stated).
//!
//! A column a sheet does not define is refused rather than ignored, so a
//! misspelt `expiry_minutes` cannot quietly become the 60-minute default.
//!
//! A sheet that is absent says nothing, and a base export (if any) decides.
//! A `SigningRules` sheet with a header and no rows says "no rules", which is
//! not the same thing, and is carried as `[]`. The two are kept apart all the
//! way through: [`PlannedSigning`] holds each as an `Option`.
//!
//! `expires_minutes`: a number is that many minutes; `null` (the workbook's
//! null token; `none` is read the same) is an explicit "requests never
//! expire" (`null` in the bundle); a blank cell is the
//! bundle's absent field, which the platform reads as
//! [`crate::signing::types::DEFAULT_EXPIRES_MINUTES`]. Values are stored as
//! configured; trustee actions' effective count and requester policy are the
//! platform's to apply ([`SigningRule::required`]) and are not rewritten here.
//!
//! No credentials of any kind belong here: these are policy enums and numbers.

use super::paths::Cell;
use super::problem::{Code, Problem};
use super::sheet::{Row, Sheet, Workbook};
use crate::signing::{SigningChecks, SigningRule};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const SHEET_SIGNING_RULES: &str = "signingrules";
pub const SHEET_SIGNING_CHECKS: &str = "signingchecks";
pub const SHEET_APPROVAL_MATRIX: &str = "approvalmatrix";
/// The trust-loss record ([`PlannedSigning::trust_not_carried`]): archive
/// member names only, one per row. A diagnostic, not policy: never built into
/// a bundle, and no certificate data.
pub const SHEET_SIGNING_NOTICES: &str = "signingnotices";
const NOTICE_COLUMNS: [&str; 1] = ["trust_not_carried"];
const MATRIX_COLUMNS: [&str; 1] = ["matrix"];

const RULE_COLUMNS: [&str; 6] = [
    "action",
    "requirement",
    "signatures",
    "requester_signing",
    "expires_minutes",
    "revision",
];
const CHECK_COLUMNS: [&str; 5] = [
    "revocation_check",
    "crl_unavailable",
    "registration",
    "post_binding",
    "revision",
];
/// Also read as an explicit "never expires", beside the workbook's `null`.
pub const NO_EXPIRY: &str = "none";

/// An event's signing configuration as a plan holds it.
///
/// Each field is `None` when the source said nothing about it, and `Some`
/// (possibly an empty list) when it did, so that a configuration opened from
/// an export, a workbook or a historical source comes back out as it went in,
/// and a client preset can never be mistaken for, or replace, one that was
/// read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedSigning {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<SigningRule>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checks: Option<SigningChecks>,
    /// The enrollment approval matrix, carried as the source stated it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_matrix: Option<Value>,
    /// Archive members with public certificate trust configuration (staff
    /// issuers, certificate authorities) that the archive this plan was
    /// opened from had and the Architect does not carry. Not policy, and never
    /// built into a bundle: kept (in the plan, and in the workbook's
    /// `SigningNotices` sheet) so the loss is reported on every validation,
    /// across a save or a workbook reopen, not only once when the archive was
    /// opened.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trust_not_carried: Vec<String>,
}

impl PlannedSigning {
    pub fn is_empty(&self) -> bool {
        self.is_policy_empty() && self.trust_not_carried.is_empty()
    }

    /// Whether this states no signing policy (rules, checks or matrix).
    pub fn is_policy_empty(&self) -> bool {
        self.rules.is_none()
            && self.checks.is_none()
            && self.approval_matrix.is_none()
    }

    /// The configuration a bundle carries, field by field.
    pub fn from_export(document: &Value) -> Result<Self, Problem> {
        let rules = match document.get("signing_rules") {
            None | Some(Value::Null) => None,
            Some(value) => Some(
                serde_json::from_value(value.clone()).map_err(|error| {
                    Problem::error(
                        Code::InvalidValue,
                        "signing_rules",
                        format!(
                            "the export's signing rules do not read: {error}"
                        ),
                    )
                    .id("signing.export-unreadable")
                })?,
            ),
        };
        let checks = match document.get("signing_checks") {
            None | Some(Value::Null) => None,
            Some(value) => Some(
                serde_json::from_value(value.clone()).map_err(|error| {
                    Problem::error(
                        Code::InvalidValue,
                        "signing_checks",
                        format!(
                            "the export's certificate checks do not read: {error}"
                        ),
                    )
                    .id("signing.export-unreadable")
                })?,
            ),
        };
        let approval_matrix = document
            .get("approval_matrix")
            .filter(|matrix| !matrix.is_null())
            .cloned();
        Ok(PlannedSigning {
            rules,
            checks,
            approval_matrix,
            trust_not_carried: Vec::new(),
        })
    }

    /// The sheets that hold this configuration, one per field that is set.
    pub fn sheets(&self) -> Vec<Sheet> {
        let mut sheets = Vec::new();
        if let Some(rules) = &self.rules {
            sheets.push(rules_sheet(rules));
        }
        if let Some(checks) = &self.checks {
            sheets.push(checks_sheet(checks));
        }
        if !self.trust_not_carried.is_empty() {
            sheets.push(grid_sheet(
                "SigningNotices",
                &NOTICE_COLUMNS,
                self.trust_not_carried
                    .iter()
                    .map(|member| vec![Cell::text(member.clone())])
                    .collect(),
            ));
        }
        if let Some(matrix) = &self.approval_matrix {
            sheets.push(grid_sheet(
                "ApprovalMatrix",
                &MATRIX_COLUMNS,
                vec![vec![Cell::text(matrix.to_string())]],
            ));
        }
        sheets
    }

    /// The configuration the workbook's sheets hold. Every row that does not
    /// read is reported; none is skipped.
    pub fn from_workbook(workbook: &Workbook) -> Result<Self, Vec<Problem>> {
        let mut problems = Vec::new();
        for (key, columns) in [
            (SHEET_SIGNING_RULES, &RULE_COLUMNS[..]),
            (SHEET_SIGNING_CHECKS, &CHECK_COLUMNS[..]),
            (SHEET_APPROVAL_MATRIX, &MATRIX_COLUMNS[..]),
            (SHEET_SIGNING_NOTICES, &NOTICE_COLUMNS[..]),
        ] {
            if let Some(sheet) = workbook.sheet(key) {
                for header in &sheet.headers {
                    if !header.is_empty() && !columns.contains(&header.as_str())
                    {
                        problems.push(
                            Problem::error(
                                Code::InvalidValue,
                                format!("{}.{header}", sheet.name),
                                format!(
                                    "'{header}' is not a {} column; the columns are {}",
                                    sheet.name,
                                    columns.join(", ")
                                ),
                            )
                            .id("signing.unknown-column"),
                        );
                    }
                }
            }
        }
        let rules = workbook.sheet(SHEET_SIGNING_RULES).map(|sheet| {
            sheet
                .rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| {
                    read_row::<SigningRule>(
                        row,
                        &format!("{}[{index}]", sheet.name),
                        &RULE_COLUMNS,
                    )
                    .map_err(|problem| problems.push(problem))
                    .ok()
                })
                .collect::<Vec<_>>()
        });
        let checks = match workbook.sheet(SHEET_SIGNING_CHECKS) {
            None => None,
            Some(sheet) if sheet.rows.len() != 1 => {
                problems.push(
                    Problem::error(
                        Code::InvalidValue,
                        sheet.name.clone(),
                        format!(
                            "certificate checks are one row, not {}",
                            sheet.rows.len()
                        ),
                    )
                    .id("signing.checks-not-one-row"),
                );
                None
            }
            Some(sheet) => read_row::<SigningChecks>(
                &sheet.rows[0],
                &format!("{}[0]", sheet.name),
                &CHECK_COLUMNS,
            )
            .map_err(|problem| problems.push(problem))
            .ok(),
        };
        let approval_matrix = match workbook.sheet(SHEET_APPROVAL_MATRIX) {
            None => None,
            Some(sheet) => {
                let cell = (sheet.rows.len() == 1)
                    .then(|| sheet.rows[0].get("matrix").cloned())
                    .flatten();
                match cell {
                    Some(Value::String(text)) => serde_json::from_str(&text)
                        .map_err(|error| {
                            problems.push(
                                Problem::error(
                                    Code::InvalidValue,
                                    format!("{}[0].matrix", sheet.name),
                                    format!("the approval matrix is not JSON: {error}"),
                                )
                                .id("signing.matrix-unreadable"),
                            )
                        })
                        .ok(),
                    Some(Value::Null) | None => {
                        problems.push(
                            Problem::error(
                                Code::InvalidValue,
                                sheet.name.clone(),
                                "the approval matrix is one row with one `matrix` cell",
                            )
                            .id("signing.matrix-not-one-cell"),
                        );
                        None
                    }
                    Some(value) => Some(value),
                }
            }
        };
        let trust_not_carried = workbook
            .rows(SHEET_SIGNING_NOTICES)
            .iter()
            .filter_map(|row| row.text("trust_not_carried"))
            .map(str::to_string)
            .collect();
        if problems.is_empty() {
            Ok(PlannedSigning {
                rules,
                checks,
                approval_matrix,
                trust_not_carried,
            })
        } else {
            Err(problems)
        }
    }
}

fn enum_text<T: Serialize>(value: &T) -> Cell {
    match serde_json::to_value(value) {
        Ok(Value::String(text)) => Cell::text(text),
        _ => Cell::Blank,
    }
}

fn rules_sheet(rules: &[SigningRule]) -> Sheet {
    let rows: Vec<Vec<Cell>> = rules
        .iter()
        .map(|rule| {
            vec![
                enum_text(&rule.action),
                enum_text(&rule.requirement),
                Cell::Int(i64::from(rule.signatures)),
                enum_text(&rule.requester_signing),
                match rule.expires_minutes {
                    Some(minutes) => Cell::Int(i64::from(minutes)),
                    None => Cell::text(super::paths::NULL_LITERAL),
                },
                Cell::Int(rule.revision),
            ]
        })
        .collect();
    grid_sheet("SigningRules", &RULE_COLUMNS, rows)
}

fn checks_sheet(checks: &SigningChecks) -> Sheet {
    grid_sheet(
        "SigningChecks",
        &CHECK_COLUMNS,
        vec![vec![
            enum_text(&checks.revocation_check),
            enum_text(&checks.crl_unavailable),
            enum_text(&checks.registration),
            enum_text(&checks.post_binding),
            Cell::Int(checks.revision),
        ]],
    )
}

fn grid_sheet(name: &str, columns: &[&str], rows: Vec<Vec<Cell>>) -> Sheet {
    let mut grid =
        vec![columns.iter().map(|column| Cell::text(*column)).collect()];
    grid.extend(rows);
    Sheet::from_grid(name, &grid)
        .expect("a sheet written from its own fixed columns reads")
}

/// One row as the bundle's JSON object, then as the bundle's type, so a value
/// the platform would refuse is refused here with the row it came from.
fn read_row<T: serde::de::DeserializeOwned>(
    row: &Row,
    at: &str,
    columns: &[&str],
) -> Result<T, Problem> {
    let mut object = Map::new();
    for column in columns {
        match row.get(column) {
            // A blank cell: the field is absent, and the bundle's default
            // applies (60 minutes for an expiry).
            None => {}
            // The workbook's null token: an explicit null, which for an
            // expiry is "never expires" and must not become the default.
            Some(Value::Null) if *column == "expires_minutes" => {
                object.insert((*column).to_string(), Value::Null);
            }
            Some(Value::Null) => {}
            Some(Value::String(text)) if text.trim().is_empty() => {}
            Some(Value::String(text))
                if *column == "expires_minutes"
                    && text.trim().eq_ignore_ascii_case(NO_EXPIRY) =>
            {
                object.insert((*column).to_string(), Value::Null);
            }
            Some(Value::String(text))
                if matches!(
                    *column,
                    "signatures" | "expires_minutes" | "revision"
                ) =>
            {
                let number: i64 = text.trim().parse().map_err(|_| {
                    Problem::error(
                        Code::InvalidValue,
                        format!("{at}.{column}"),
                        format!("'{text}' is not a whole number"),
                    )
                    .id("signing.not-a-number")
                })?;
                object.insert((*column).to_string(), Value::from(number));
            }
            // A spreadsheet stores every number as a float; a whole one is
            // the integer it shows.
            Some(Value::Number(number))
                if number.is_f64()
                    && number.as_f64().is_some_and(|f| f.fract() == 0.0) =>
            {
                let whole = number.as_f64().unwrap_or_default() as i64;
                object.insert((*column).to_string(), Value::from(whole));
            }
            Some(value) => {
                object.insert((*column).to_string(), value.clone());
            }
        }
    }
    serde_json::from_value(Value::Object(object)).map_err(|error| {
        Problem::error(
            Code::InvalidValue,
            at.to_string(),
            format!("this signing setting does not read: {error}"),
        )
        .id("signing.row-unreadable")
    })
}

#[cfg(test)]
#[path = "signing_sheet_tests.rs"]
mod tests;
