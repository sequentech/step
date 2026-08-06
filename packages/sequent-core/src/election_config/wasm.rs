// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The browser's view of this module.
//!
//! Everything here is a thin wrapper: the work happens in the same functions
//! `step-cli` and windmill call, and this only converts between them and
//! JavaScript. That is the whole point — a file that validates in a browser
//! imports on the server, because the same code decided both times.
//!
//! Two entry points, matching the two questions a front end asks:
//!
//! * [`check_bundle`] — "would the platform accept this export?" Enough on its own
//!   for a page that only checks an existing file, and it needs no template engine
//!   or spreadsheet parser.
//! * [`build_from_workbook`] — "turn this spreadsheet into something importable."
//!   Returns the files to offer as downloads, or the problems to show instead.
//!
//! Nothing here touches a network or a filesystem, so a client's census never
//! leaves the browser. That is not a side effect of the design; it is the reason
//! for it.

use crate::election_config::problem::{Code, Problem, Report};
use crate::election_config::schema::ImportElectionEventSchema;
use crate::election_config::validate;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[cfg(feature = "election_config_archive")]
use crate::election_config::render::TemplateSet;
#[cfg(feature = "election_config_xlsx")]
use crate::election_config::xlsx::read_xlsx;
#[cfg(feature = "election_config_archive")]
use crate::election_config::{archive, build, presets, BuildOptions};

#[wasm_bindgen(typescript_custom_section)]
const ITYPES: &'static str = r#"
export type Severity = "error" | "warning";

export interface Problem {
    severity: Severity;
    /** Stable identifier, safe to match on. Message wording is not. */
    code: string;
    /** Where: a bundle path, or a sheet and row for a source document. */
    path: string;
    message: string;
    external_id?: string;
}

export interface Report {
    problems: Problem[];
}

/** One file to offer as a download. */
export interface Artifact {
    name: string;
    bytes: Uint8Array;
}

export interface BuildOutput {
    /** Empty when the build failed; check `report` first. */
    importable: Artifact[];
    /** Files that must NOT go inside the archive. */
    auxiliary: Artifact[];
    /** The importable archive, ready to download. */
    archive?: Artifact;
    /** Everything found, errors and warnings together. */
    report: Report;
    /** Absent when the build failed. */
    event_external_id?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Report")]
    pub type IReport;

    #[wasm_bindgen(typescript_type = "BuildOutput")]
    pub type IBuildOutput;
}

/// Check an existing export, the way the server would.
///
/// Takes the `export_election_event-<id>.json` document as text. Returns the same
/// [`Report`] the importer produces, so a page can show the problems before anyone
/// uploads anything.
#[wasm_bindgen(js_name = checkBundle)]
pub fn check_bundle(document: &str) -> Result<IReport, JsError> {
    let bundle: ImportElectionEventSchema = serde_json::from_str(document)
        .map_err(|error| {
            // A parse failure is itself a finding, but it is not a Report — the
            // caller has nothing to render a list from, so it is an exception.
            JsError::new(&format!(
                "this is not an election event export: {error}"
            ))
        })?;

    to_js(&validate(&bundle)).map(IReport::from)
}

/// The authentication presets a front end can offer.
///
/// Returned rather than duplicated in TypeScript, so a dropdown cannot list a
/// preset that does not exist or miss one that does.
#[cfg(feature = "election_config_archive")]
#[wasm_bindgen(js_name = authPresets)]
pub fn auth_presets() -> Result<JsValue, JsError> {
    #[derive(Serialize)]
    struct Listed {
        name: &'static str,
        summary: &'static str,
        uses_otp: bool,
        required_parameters: Vec<&'static str>,
        optional_parameters: Vec<&'static str>,
    }

    let listed: Vec<Listed> = presets::PRESETS
        .iter()
        .map(|preset| Listed {
            name: preset.name,
            summary: preset.summary,
            uses_otp: preset.uses_otp,
            required_parameters: preset.required_parameters.to_vec(),
            optional_parameters: preset.optional_parameters.to_vec(),
        })
        .collect();
    to_js(&listed)
}

/// Build an importable bundle from a workbook the user picked.
///
/// `workbook` is the bytes of an `.xlsx` — what a file input hands over. The
/// options are all optional: `tenantId`, `baseExport` (a parsed export document),
/// `slug`, `createdAt` and `authPreset`.
///
/// Returns the files to download and everything found. On failure the files are
/// empty and the report says why, rather than throwing: a list of problems is
/// something a page can render, and an exception is not.
#[cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_archive"
))]
#[wasm_bindgen(js_name = buildFromWorkbook)]
pub fn build_from_workbook(
    workbook: &[u8],
    options: JsValue,
) -> Result<IBuildOutput, JsError> {
    #[derive(serde::Deserialize, Default)]
    #[serde(default, rename_all = "camelCase")]
    struct Options {
        tenant_id: Option<String>,
        base_export: Option<serde_json::Value>,
        slug: Option<String>,
        created_at: Option<String>,
        auth_preset: Option<String>,
    }

    let options: Options = if options.is_undefined() || options.is_null() {
        Options::default()
    } else {
        serde_wasm_bindgen::from_value(options)
            .map_err(|error| JsError::new(&format!("bad options: {error}")))?
    };

    let read = match read_xlsx(workbook) {
        Ok(read) => read,
        Err(problem) => return failed(problem).map(IBuildOutput::from),
    };

    let templates = match TemplateSet::builtin() {
        Ok(templates) => templates,
        Err(problem) => return failed(problem).map(IBuildOutput::from),
    };

    let built = build(
        &read,
        &templates,
        &BuildOptions {
            tenant_id: options.tenant_id,
            base_export: options.base_export,
            slug: options.slug,
            created_at: options.created_at,
            auth_preset: options.auth_preset,
        },
    );

    let bundle = match built {
        Ok(bundle) => bundle,
        Err(report) => {
            return to_js(&Output::refused(report)).map(IBuildOutput::from)
        }
    };

    // The same second pass `step-cli` makes: a document can be internally
    // consistent and still describe an event the platform would reject.
    let mut report = bundle.warnings.clone();
    match serde_json::from_value::<ImportElectionEventSchema>(
        bundle.export.clone(),
    ) {
        Ok(schema) => {
            for problem in validate(&schema).problems {
                report.push(problem);
            }
        }
        Err(error) => report.push(Problem::error(
            Code::InvalidValue,
            "bundle",
            format!(
                "the built bundle does not match the import schema, which is a \
                 bug in this tool rather than in the workbook: {error}"
            ),
        )),
    }

    if report.has_errors() {
        return to_js(&Output::refused(report)).map(IBuildOutput::from);
    }

    let layout = archive::layout(&bundle);
    let zipped = match archive::zip(&layout.importable) {
        Ok(bytes) => bytes,
        Err(problem) => return failed(problem).map(IBuildOutput::from),
    };

    to_js(&Output {
        importable: layout.importable.iter().map(File::from).collect(),
        auxiliary: layout.auxiliary.iter().map(File::from).collect(),
        archive: Some(File {
            name: layout.archive_name,
            bytes: zipped,
        }),
        report,
        event_external_id: Some(bundle.event_external_id.clone()),
    })
    .map(IBuildOutput::from)
}

#[derive(Serialize)]
struct File {
    name: String,
    bytes: Vec<u8>,
}

#[cfg(feature = "election_config_archive")]
impl From<&archive::Artifact> for File {
    fn from(artifact: &archive::Artifact) -> Self {
        File {
            name: artifact.name.clone(),
            bytes: artifact.bytes.clone(),
        }
    }
}

#[derive(Serialize)]
struct Output {
    importable: Vec<File>,
    auxiliary: Vec<File>,
    #[serde(skip_serializing_if = "Option::is_none")]
    archive: Option<File>,
    report: Report,
    #[serde(skip_serializing_if = "Option::is_none")]
    event_external_id: Option<String>,
}

impl Output {
    /// A build that produced no files, and the reasons.
    fn refused(report: Report) -> Self {
        Output {
            importable: Vec::new(),
            auxiliary: Vec::new(),
            archive: None,
            report,
            event_external_id: None,
        }
    }
}

/// One problem, as an output with nothing in it.
fn failed(problem: Problem) -> Result<JsValue, JsError> {
    let mut report = Report::default();
    report.push(problem);
    to_js(&Output::refused(report))
}

/// Plain JS values, not wasm-bindgen classes.
///
/// A front end holds these in state, passes them to React, and serialises them;
/// an opaque handle with a `free()` method would be a memory leak waiting for
/// whoever forgets to call it.
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    serde_wasm_bindgen::to_value(value)
        .map_err(|error| JsError::new(&format!("could not convert: {error}")))
}
