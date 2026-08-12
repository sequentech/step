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
    /// An `export_election_event-<id>.json`, as the Admin Portal writes it.
    ///
    /// Spelled with a hyphen rather than run together, because this is a wire
    /// value a client reads: it appears in the wizard's own sentence about what
    /// was opened.
    #[serde(rename = "election-event")]
    ElectionEvent,
    /// The whole election-event export, zipped — the JSON plus its voters,
    /// images and support-material files.
    ///
    /// Told apart from the bare JSON because it carries more: only this door can
    /// fill in a census or a candidate's photograph, so the wizard can say why a
    /// screen is empty when it was the other one.
    #[serde(rename = "election-event-archive")]
    ElectionEventArchive,
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
    open_named(bytes, None)
}

/// Whether a name is an encrypted Admin Portal export.
///
/// **The only thing about an encrypted file that is not encrypted is its name.**
/// An `.ezip` is AES-CBC from its first byte, so it has no `PK` magic and no valid
/// UTF-8; without this it would come back as "neither a .zip, an .xlsx, nor text",
/// which sends somebody hunting for a corrupted download when what they need is to
/// ask for the unencrypted export. The bytes cannot be checked, so this is a guess
/// — but a guess that only ever *improves* a refusal is worth making, and it is
/// never reached for a file that opened.
fn looks_encrypted(name: Option<&str>) -> bool {
    name.map(str::to_ascii_lowercase)
        .is_some_and(|name| name.ends_with(".ezip"))
}

/// The fingerprint of an election event export.
///
/// **Checked before the plan attempt, and that ordering is load-bearing.** A
/// `Blueprint` requires only `version` and `external_id`, and every other field
/// defaults — so serde is not a reliable discriminator between the two documents,
/// and an export that failed the plan parse would be refused with a message about
/// a plan file rather than opened.
///
/// Several keys together rather than one, because any single one of them could
/// plausibly appear in something else. All of these are required by
/// `ImportElectionEventSchema`, so a document missing one would not import into the
/// platform either.
fn looks_like_an_event(document: &serde_json::Value) -> bool {
    document
        .get("election_event")
        .is_some_and(|event| event.is_object())
        && [
            "elections",
            "contests",
            "candidates",
            "areas",
            "area_contests",
        ]
        .iter()
        .all(|key| document.get(key).is_some_and(|value| value.is_array()))
}

/// Open whatever this is, with the file's name to fall back on.
///
/// The name is only ever used to make a *refusal* more useful — nothing that
/// opens depends on it, so a renamed file still opens as what it is.
pub fn open_named(bytes: &[u8], name: Option<&str>) -> Result<Opened, Report> {
    if !is_zip(bytes) {
        if looks_encrypted(name) {
            return Err(refuse(
                "this looks like an encrypted election event export. Its \
                 contents are encrypted, so nothing here can read them — ask \
                 whoever exported it for the unencrypted .zip.",
            ));
        }
        let text = std::str::from_utf8(bytes).map_err(|_| {
            refuse(
                "this is not a configuration: it is neither a .zip, an .xlsx, \
                 nor text",
            )
        })?;

        // An election event export, before the plan attempt — see
        // `looks_like_an_event`.
        if let Ok(document) = serde_json::from_str::<serde_json::Value>(text) {
            if looks_like_an_event(&document) {
                let read = super::plan_from_event::plan_from_event(&document)?;
                return Ok(Opened {
                    plan: read.plan,
                    report: read.report,
                    source: Source::ElectionEvent,
                });
            }
        }

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

    // An election event export, zipped. Third rather than first: a delivery
    // carries one of these *inside* `official_election_setup.zip`, which `open`
    // never unpacks, so the two cannot collide at the root — but keeping
    // `blueprint.json` ahead of it means a delivery is still a delivery even if
    // somebody adds an export beside the plan one day.
    //
    // `contains` rather than an exact name, because the document is named for the
    // event's identifier and the Admin Portal's own importer matches it the same
    // way — including in a subdirectory.
    if let Some(member) = names
        .iter()
        .find(|name| name.contains(EVENT_MEMBER) && name.ends_with(".json"))
    {
        let raw = entry(bytes, member)?;
        let document: serde_json::Value = serde_json::from_slice(&raw)
            .map_err(|error| {
                refuse(format!(
                    "`{member}` inside this .zip is not readable JSON: {error}"
                ))
            })?;
        let mut read = super::plan_from_event::plan_from_event(&document)?;

        // And everything that travelled beside it. The census, the candidates'
        // photographs and the support-material files are in the archive and not in
        // the document, so this is the only door that can fill them in — which is
        // the whole reason the two are told apart rather than reported as one kind.
        let beside = super::plan_from_event::Beside {
            voters: names
                .iter()
                .find(|name| {
                    name.contains(VOTERS_MEMBER) && name.ends_with(".csv")
                })
                .and_then(|name| entry(bytes, name).ok())
                .and_then(|raw| String::from_utf8(raw).ok()),
            files: names
                .iter()
                .filter(|name| {
                    name.starts_with("images/")
                        || name.starts_with("export_S3_files/")
                })
                .filter_map(|name| {
                    entry(bytes, name).ok().map(|bytes| (name.clone(), bytes))
                })
                .collect(),
        };
        super::plan_from_event::fill_from_archive(
            &mut read.plan,
            &document,
            &beside,
            &mut read.report,
        );

        return Ok(Opened {
            plan: read.plan,
            report: read.report,
            source: Source::ElectionEventArchive,
        });
    }

    // Named, because "that is not a configuration" about a zip somebody is sure
    // is the right one is a dead end. An empty archive gets its own sentence
    // rather than a list with nothing after the colon.
    Err(refuse(if names.is_empty() {
        "this .zip is neither a delivery, a workbook nor an election event \
         export: it is empty"
            .to_string()
    } else {
        format!(
            "this .zip is neither a delivery, a workbook nor an election event \
             export. It contains: {}",
            names.join(", ")
        )
    }))
}

fn one(problem: Problem) -> Report {
    let mut report = Report::default();
    report.push(problem);
    report
}

/// The stem of the export document's name, as the Admin Portal writes it.
///
/// Not a whole file name: the document is `export_election_event-<uuid>.json`, and
/// `import_election_event.rs` matches it by this same fragment.
const EVENT_MEMBER: &str = "export_election_event";

/// The stem of the census member's name. `export_voters-<uuid>.csv`.
const VOTERS_MEMBER: &str = "export_voters";

/// One entry's bytes, by name.
fn entry(bytes: &[u8], name: &str) -> Result<Vec<u8>, Report> {
    let mut reader = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|error| {
            refuse(format!("this .zip could not be opened: {error}"))
        })?;
    let mut member = reader.by_name(name).map_err(|error| {
        refuse(format!(
            "`{name}` could not be read from this .zip: {error}"
        ))
    })?;
    let mut out = Vec::new();
    std::io::Read::read_to_end(&mut member, &mut out).map_err(|error| {
        refuse(format!(
            "`{name}` could not be read from this .zip: {error}"
        ))
    })?;
    Ok(out)
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
