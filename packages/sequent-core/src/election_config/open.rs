// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! One door: a delivery zip, a plan `.json`, or a workbook `.xlsx`.
//!
//! **The sniffing is here rather than in a front end, and it has to be.** An
//! `.xlsx` *is* a zip — same `PK` magic — so a front end checking magic bytes hands
//! a spreadsheet to the delivery reader, which looks for `blueprint.json`, does not
//! find one, and reports a broken delivery. Telling them apart means opening the
//! archive and looking at what is inside, which is the archive reader's job.
//!
//! It also removes the last piece of format knowledge from TypeScript: the wizard
//! used to `JSON.parse(...) as Blueprint`, an unvalidated cast that would accept
//! any object at all and fail later, on a screen, with a message about something
//! else.

use crate::election_config::architect::Blueprint;
use crate::election_config::plan_from_workbook::plan_from_workbook;
use crate::election_config::problem::{Code, Problem, Report};
use crate::election_config::xlsx::read_xlsx;

/// What the bytes turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// The `.zip` a build hands over, with `blueprint.json` inside it.
    Delivery,
    /// A bare plan, saved on its own.
    Plan,
    /// An import workbook — the janitor's format, or the one in a delivery.
    Workbook,
}

/// A plan, whatever it arrived as.
#[derive(Debug, Clone)]
pub struct Opened {
    pub plan: Blueprint,
    /// Warnings. Errors come back as `Err(Report)`.
    pub report: Report,
    pub source: Source,
}

/// The three `PK` signatures a zip can start with.
///
/// All three, because checking only `\x03\x04` sent an *empty* archive to
/// `JSON.parse`, which reported `Unexpected token 'P'` — a message about the wrong
/// thing entirely.
fn is_zip(bytes: &[u8]) -> bool {
    matches!(
        bytes.get(..4),
        Some(b"PK\x03\x04") | Some(b"PK\x05\x06") | Some(b"PK\x07\x08")
    )
}

fn refuse(message: impl Into<String>) -> Report {
    let mut report = Report::default();
    report.push(Problem::error(Code::InvalidValue, "file", message));
    report
}

/// Open whatever this is.
///
/// The order is forced by the formats themselves: a delivery and a workbook are
/// both zips, so the archive has to be opened and inspected. A delivery carries
/// `blueprint.json`; a spreadsheet carries `[Content_Types].xml`. Anything that is
/// not a zip is tried as a plan.
pub fn open(bytes: &[u8]) -> Result<Opened, Report> {
    if !is_zip(bytes) {
        let text = std::str::from_utf8(bytes).map_err(|_| {
            refuse(
                "this is not a configuration: it is neither a .zip, an .xlsx, \
                 nor text",
            )
        })?;
        let plan: Blueprint = serde_json::from_str(text).map_err(|error| {
            refuse(format!(
                "this is not a plan file. If you have the delivery .zip, open \
                 that instead. ({error})"
            ))
        })?;
        return Ok(Opened {
            plan,
            report: Report::default(),
            source: Source::Plan,
        });
    }

    let names = members(bytes)?;

    if names.iter().any(|name| name == super::archive::PLAN_MEMBER) {
        let raw = super::archive::plan_in_delivery(bytes)
            .map_err(|problem| one(problem))?;
        let plan: Blueprint =
            serde_json::from_slice(&raw).map_err(|error| {
                refuse(format!(
                    "the plan inside this delivery could not be read: {error}"
                ))
            })?;
        return Ok(Opened {
            plan,
            report: Report::default(),
            source: Source::Delivery,
        });
    }

    if names.iter().any(|name| name == "[Content_Types].xml") {
        let workbook = read_xlsx(bytes).map_err(one)?;
        let read = plan_from_workbook(&workbook)?;
        return Ok(Opened {
            plan: read.plan,
            report: read.report,
            source: Source::Workbook,
        });
    }

    // Named, because "that is not a configuration" about a zip somebody is sure
    // is the right one is a dead end.
    Err(refuse(format!(
        "this .zip is neither a delivery nor a workbook. It contains: {}",
        names.join(", ")
    )))
}

fn one(problem: Problem) -> Report {
    let mut report = Report::default();
    report.push(problem);
    report
}

/// What is in the archive, by name.
fn members(bytes: &[u8]) -> Result<Vec<String>, Report> {
    let reader =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|error| {
            refuse(format!("this .zip could not be opened: {error}"))
        })?;
    Ok(reader.file_names().map(str::to_string).collect())
}

#[cfg(test)]
#[path = "open_tests.rs"]
mod open_tests;
