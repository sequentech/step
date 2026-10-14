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
use crate::election_config::sources::Sources;
use crate::election_config::xlsx::read_xlsx;

/// What the bytes turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// The `.zip` a build hands over, with `blueprint.json` inside it.
    Delivery,
    /// A bare plan, saved on its own.
    ///
    /// Still opened, and always will be — somebody has one of these saved from
    /// last month. It is no longer what the wizard *writes*, because a plan on its
    /// own is a plan with the members' names and the candidates' photographs
    /// missing.
    Plan,
    /// The zip the wizard saves: the plan, its census and its files.
    #[serde(rename = "plan-archive")]
    PlanArchive,
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

    /// What travelled beside the plan.
    ///
    /// Derived from the plan's own fields for every door that has nothing else to
    /// offer, and read from the archive for the two that do. A caller can hand this
    /// straight to `compile_plan` without asking which kind of file it opened,
    /// which is the point: the difference between a delivery and a bare JSON stops
    /// being the caller's problem.
    pub sources: Sources,
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
                    sources: Sources::from_plan(&read.plan),
                    plan: read.plan,
                    report: read.report,
                    source: Source::ElectionEvent,
                });
            }
        }

        // Through `read_plan`, so an older plan is migrated rather than read as
        // though it were current. It was not, and a version 2 plan opened here
        // kept its area *names* in the field the builder reads as identifiers.
        let read = super::architect::read_plan(text).map_err(|problem| {
            refuse(format!(
                "this is not a plan file. If you have the delivery .zip, open \
                 that instead. ({})",
                problem.message
            ))
        })?;
        return Ok(Opened {
            plan: read.plan,
            sources: read.sources,
            report: Report::default(),
            source: Source::Plan,
        });
    }

    let names = members(bytes)?;

    if names.iter().any(|name| name == super::archive::PLAN_MEMBER) {
        return open_delivery(bytes, &names);
    }

    if names.iter().any(|name| name == "[Content_Types].xml") {
        let workbook = read_xlsx(bytes).map_err(one)?;
        let read = plan_from_workbook(&workbook)?;
        return Ok(Opened {
            sources: read.sources,
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
        let mut sources = super::plan_from_event::fill_from_archive(
            &mut read.plan,
            &document,
            &beside,
            &mut read.report,
        );
        // The photographs and the support materials are still fields of the plan,
        // so the bytes half of the shim still applies. The census is not.
        sources.files = Sources::from_plan(&read.plan).files;

        return Ok(Opened {
            sources,
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

/// A save file or a delivery — one reader, because they are one layout twice.
///
/// Both carry `blueprint.json` at the root. What differs is where the bulk sits: a
/// save file keeps `census.csv` and `files/<name>` beside the plan, and a delivery
/// keeps `export_voters-<id>.csv`, `images/…` and `export_S3_files/…` **inside**
/// `official_election_setup.zip`. So the root is looked at first and the importable
/// zip second, and the answer is a `Sources` either way.
///
/// **The delivery branch used to return `Report::default()` and nothing else.** It
/// read `blueprint.json` and stopped, so reopening a delivery gave back a plan whose
/// census and photographs were whatever the JSON happened to still carry — which,
/// once the plan stops carrying them, is nothing at all. That is the defect this
/// closes, and it is invisible today precisely because the duplication is still
/// there.
fn open_delivery(bytes: &[u8], names: &[String]) -> Result<Opened, Report> {
    let raw = entry(bytes, super::archive::PLAN_MEMBER)?;
    let document: serde_json::Value =
        serde_json::from_slice(&raw).map_err(|error| {
            refuse(format!("the plan in this zip could not be read: {error}"))
        })?;
    let read = super::architect::read_plan_value(document)
        .map_err(|problem| refuse(problem.message))?;
    let plan = read.plan;

    let mut report = Report::default();

    // A name is what a plan points at, so the areas resolve before a row is read.
    let by_area_name: std::collections::BTreeMap<String, String> = plan
        .areas
        .iter()
        .map(|area| (area.name.clone(), area.external_id.clone()))
        .collect();

    // **Which of the two this is, decided by the nested zip and not by the census.**
    // A delivery holds `official_election_setup.zip`; a save file does not. Asking
    // whether `census.csv` is present looked equivalent and is not: a plan with no
    // members saves as a zip with a single member, and reading that as a delivery
    // makes the wizard's own save file the one file it cannot recognise. The
    // contract check caught it on the first plan it tried.
    let saved = !names
        .iter()
        .any(|name| name == super::archive::IMPORTABLE_MEMBER);
    let (census_text, files, source) = if saved {
        (
            entry(bytes, super::archive::CENSUS_MEMBER)
                .ok()
                .and_then(|raw| String::from_utf8(raw).ok()),
            names
                .iter()
                .filter(|name| {
                    name.starts_with(super::archive::FILES_PREFIX)
                        && !name.ends_with('/')
                })
                .filter_map(|name| {
                    let short = name
                        .strip_prefix(super::archive::FILES_PREFIX)?
                        .to_string();
                    entry(bytes, name).ok().map(|bytes| (short, bytes))
                })
                .collect::<Vec<_>>(),
            Source::PlanArchive,
        )
    } else {
        // A delivery: everything worth having is a member of the nested importable
        // zip, so it has to be opened in turn.
        let inner = names
            .iter()
            .find(|name| name.as_str() == super::archive::IMPORTABLE_MEMBER)
            .and_then(|name| entry(bytes, name).ok());
        match inner {
            Some(inner) => {
                let within = members(&inner).unwrap_or_default();
                let census = within
                    .iter()
                    .find(|name| {
                        name.contains(VOTERS_MEMBER) && name.ends_with(".csv")
                    })
                    .and_then(|name| entry(&inner, name).ok())
                    .and_then(|raw| String::from_utf8(raw).ok());
                // Keyed by the plan's own file name rather than the archive's
                // entry name: `images/document_<id>_<name>` is how the platform
                // stores it, and `<name>` is what the plan points at.
                let files = within
                    .iter()
                    .filter(|name| {
                        name.starts_with("images/")
                            || name.starts_with("export_S3_files/")
                    })
                    .filter_map(|name| {
                        let short = plan_file_name(name)?;
                        entry(&inner, name).ok().map(|bytes| (short, bytes))
                    })
                    .collect::<Vec<_>>();
                (census, files, Source::Delivery)
            }
            None => {
                report.push(
                    Problem::warning(
                        Code::MissingField,
                        "delivery",
                        format!(
                            "this zip has a plan but no \
                             {}, so the census and the files it names are not \
                             in it.",
                            super::archive::IMPORTABLE_MEMBER
                        ),
                    )
                    .id("delivery.no-importable"),
                );
                (None, Vec::new(), Source::Delivery)
            }
        }
    };

    // A password this plan's own recipe generated is regenerated on the next
    // build, so reading it back would leave the reopened plan carrying two answers
    // to one question — and `check_passwords` refuses that, which made a delivery
    // of generated passwords the one kind that could not be re-imported. A
    // password a client *typed* has no recipe behind it and is kept.
    let ours: &[&str] =
        if plan.passwords.as_ref().is_some_and(|recipe| recipe.ready()) {
            &[super::password::COLUMN]
        } else {
            &[]
        };

    let census = match census_text {
        Some(text) => {
            match super::sources::CsvCensus::ignoring(&text, by_area_name, ours)
            {
                Ok(census) => {
                    for note in census.notes() {
                        report.push(
                            Problem::warning(
                                Code::InvalidValue,
                                "voters",
                                note,
                            )
                            .id("census.note"),
                        );
                    }
                    Some(std::sync::Arc::new(census)
                        as std::sync::Arc<dyn super::sources::CensusSource>)
                }
                Err(why) => {
                    report.push(
                        Problem::warning(
                            Code::InvalidValue,
                            "voters",
                            format!(
                            "the census in this zip could not be read: {why}"
                        ),
                        )
                        .id("census.unreadable-member"),
                    );
                    None
                }
            }
        }
        None => None,
    };

    // Nothing beside the plan means the plan is all there is, which is what a
    // delivery written before this change looks like.
    let sources = if census.is_none() && files.is_empty() {
        read.sources
    } else {
        Sources {
            census,
            files: files
                .into_iter()
                .map(|(name, bytes)| (name, std::sync::Arc::from(bytes)))
                .collect(),
        }
    };

    Ok(Opened {
        plan,
        report,
        source,
        sources,
    })
}

/// The name a plan would use for an archive entry.
///
/// The platform stores a document as `images/document_<uuid>_<name>`, and the plan
/// points at `<name>` — so the identifier has to come off, and only the *first* two
/// underscore-separated pieces belong to it. A file called `photo_of_ada.jpg` keeps
/// every underscore it came with.
fn plan_file_name(entry: &str) -> Option<String> {
    let base = entry.rsplit('/').next()?;
    let rest = base.strip_prefix("document_")?;
    let (_, name) = rest.split_once('_')?;
    (!name.is_empty()).then(|| name.to_string())
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
