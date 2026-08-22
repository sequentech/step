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
//! Five entry points, matching the questions a front end asks:
//!
//! * [`check_bundle`] — "would the platform accept this export?" Enough on its own
//!   for a page that only checks an existing file, and it needs no template engine
//!   or spreadsheet parser.
//! * [`build_from_workbook`] — "turn this spreadsheet into something importable."
//!   Returns the files to offer as downloads, or the problems to show instead.
//! * [`validate_plan_js`] — "is there anything wrong with this plan?", asked of a
//!   wizard's document rather than of a built bundle.
//! * [`compile_plan_js`] — "turn this plan into something importable." The same
//!   output shape as `build_from_workbook`, because a wizard and a spreadsheet
//!   produce the same thing.
//! * [`preview_ballot_js`] — "what will voters actually see?" The ballots the
//!   platform's own builder makes of the compiled entities, in the shape the
//!   Voting Portal already opens.
//!
//! The last three are gated on `election_config_archive` alone. Compiling a plan
//! needs the templates, the builder and the zip writer, but no spreadsheet
//! parser — so the wizard's package does not carry calamine.
//!
//! Nothing here touches a network or a filesystem, so a client's census never
//! leaves the browser. That is not a side effect of the design; it is the reason
//! for it.

use crate::election_config::census_csv;
use crate::election_config::fixtures;
use crate::election_config::problem::{Code, Problem, Report};
use crate::election_config::schema::ImportElectionEventSchema;
use crate::election_config::sources::{self, Sources};
use crate::election_config::validate;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[cfg(feature = "election_config_archive")]
use crate::election_config::render::TemplateSet;
#[cfg(feature = "election_config_xlsx")]
use crate::election_config::xlsx::read_xlsx;
#[cfg(feature = "election_config_archive")]
use crate::election_config::{
    architect, archive, build, presets, preview, profile, BuildOptions,
};

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
    /**
     * Where in a source spreadsheet, structurally — so a screen can group by tab
     * and point at a cell rather than parsing `path` back apart. Absent for a
     * problem about a plan or a bundle, neither of which has a row 12.
     */
    at?: {sheet: string; row?: number; column?: string};
}

export interface Report {
    problems: Problem[];
}

/**
 * What `openConfiguration` made of the bytes it was given.
 *
 * `plan` is null when nothing could be read, and `report` says why — a report
 * rather than a thrown error, because a screen can group one by tab and point at
 * cells with it.
 */
export interface Opened {
    plan: unknown | null;
    report: Report;
    source?:
        | "delivery"
        | "plan"
        | "workbook"
        | "election-event"
        | "election-event-archive";
}

/** The verdict every caller of the validator must reach on a fixture. */
export interface Expect {
    errors: string[];
    warnings: string[];
}

/** A bundle with a known verdict, for a front end's own test suite. */
export interface FixtureCase {
    name: string;
    /** Why the case exists. Read this before changing what it expects. */
    why: string;
    bundle: unknown;
    expect: Expect;
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
    /**
     * The whole delivery: a zip that is *not* importable, holding one that is.
     *
     * `official_election_setup.zip` nested beside the reopenable plan and the files a
     * person needs. Only a plan build produces one.
     */
    delivery?: Artifact;

    /** The importable archive, ready to download. */
    archive?: Artifact;
    /** Everything found, errors and warnings together. */
    report: Report;
    /** Absent when the build failed. */
    event_external_id?: string;
}

/**
 * The ballots a plan's voters would be given.
 *
 * The same five keys, in the same shape, as the publication preview the platform
 * writes to its public bucket — so the Voting Portal itself can open this
 * document, and what it draws is not our idea of a ballot but its own.
 *
 * `ballot_styles` holds one entry per area and election, each of them the
 * `ballot_eml` a `ballot_style` row carries. Its type is `IBallotStyle` from
 * `@sequentech/ui-core`, which this package does not depend on, so it is left as
 * `unknown` rather than described a second time and allowed to drift.
 */
export interface BallotPreview {
    ballot_styles: unknown[];
    election_event: unknown;
    elections: unknown[];
    support_materials: unknown[];
    documents: unknown[];
}

export interface PreviewOutput {
    /** Absent when the plan could not be compiled; read `report`. */
    preview?: BallotPreview;
    /**
     * The areas those ballots belong to, named.
     *
     * Not inside `preview`, on purpose: a ballot style names its area by id and
     * that is exactly what the platform writes, so adding a key would stop the
     * document being the same file. This sits beside it because a picker
     * offering four uuids is not a choice anybody can make.
     */
    areas: Array<{id: string; name: string}>;
    /**
     * The elections those ballots belong to, named, for the same reason.
     *
     * A voter is handed one area's ballot for one election, so the preview shows
     * one of each — every election at once is a page nobody ever sees. An
     * election carries no `name` column; the platform keeps it under
     * `presentation.i18n.<lang>.name`, and this falls back to `external_id`,
     * which is a word the author typed.
     */
    elections: Array<{id: string; name: string}>;
    /** Everything found on the way, errors and warnings together. */
    report: Report;
}

/**
 * A census the host holds, read a batch at a time.
 *
 * The reason this is an interface and not an array: a census can be ten million
 * members, and handing that over as one value means holding it twice while the
 * boundary copies it. So the core pulls, and the rows are alive only for as long
 * as it takes to turn one batch into voters.
 *
 * `CensusCsvReader` implements it as it stands — deliberately. A CSV the browser
 * has already parsed should not be parsed again to be handed over.
 */
export interface CensusPull {
    /** The columns a row will have, in order. Answerable before any row. */
    columns(): string[];
    /** Back to the first row; one compile reads the census more than once. */
    rewind(): void;
    /** The next `size` rows, aligned to `columns()`. Empty when done. */
    nextBatch(size: number): string[][];
}

/**
 * What `compilePlan` and `previewBallot` accept beside the plan.
 *
 * Both fields are optional and both are new. Passing neither is what every caller
 * did until now and still means the same thing: the census and the files are read
 * off the plan itself.
 */
export interface CompileOptions {
    /** Where the voters come from, instead of `plan.voters`. */
    census?: CensusPull;
    /**
     * The bytes the plan's file names refer to — a logo, a candidate's
     * photograph, a support material. Keyed by the name the plan carries.
     */
    files?: Record<string, Uint8Array>;
}

/**
 * What the bytes handed to `openFile` turned out to be.
 *
 * `plan-archive` is the wizard's own save file. `delivery` is what a build hands
 * over. `plan` is a bare `blueprint.json`, which still opens and no longer gets
 * written.
 */
export type OpenedSource =
    | "delivery"
    | "plan"
    | "plan-archive"
    | "workbook"
    | "election-event"
    | "election-event-archive";
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Report")]
    pub type IReport;

    #[wasm_bindgen(typescript_type = "BuildOutput")]
    pub type IBuildOutput;

    #[wasm_bindgen(typescript_type = "PreviewOutput")]
    pub type IPreviewOutput;
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

/// The bundles a front end's own tests should agree with.
///
/// The same list the Rust tests run, handed over as data rather than reimplemented
/// in TypeScript. A front end asserting `checkBundle(case.bundle)` matches
/// `case.expect` is checking that the browser and the server reach the same verdict
/// — which is the only thing that makes one validator worth having. A suite written
/// separately would prove only that each side agrees with itself.
#[wasm_bindgen(js_name = fixtureCases)]
pub fn fixture_cases() -> Result<JsValue, JsError> {
    to_js(&fixtures::cases())
}

/// The authentication presets a front end can offer.
///
/// **The list comes from a profile, not from Rust.** A client's identity provider
/// is theirs, so the four Sequent ships are a default rather than a ceiling —
/// pass the profile document a build loaded and get that client's own flows;
/// pass nothing and get the shipped four out of `default_profile.json`.
///
/// Returned rather than duplicated in TypeScript, so a dropdown cannot list a
/// flow the platform cannot provision or miss one it can.
#[cfg(feature = "election_config_archive")]
#[wasm_bindgen(js_name = authPresets)]
pub fn auth_presets(profile: JsValue) -> Result<JsValue, JsError> {
    let listed = presets_for(profile)?;
    to_js(&listed)
}

/// The shipped profile, as the document a profile author would edit.
///
/// Handed over whole rather than as a list of presets: a build with no
/// `?profile=` is running *on* this profile, and a front end that fetched a
/// profile and a front end that did not should be holding the same kind of thing
/// afterwards. It is also what a profile author copies from — a default nobody
/// can read is a default nobody can override deliberately.
#[cfg(feature = "election_config_archive")]
#[wasm_bindgen(js_name = defaultProfile)]
pub fn default_profile() -> Result<JsValue, JsError> {
    let document: serde_json::Value = serde_json::from_str(
        profile::DEFAULT_PROFILE_JSON,
    )
    .map_err(|error| {
        JsError::new(&format!("the shipped profile could not be read: {error}"))
    })?;
    to_js(&document)
}

/// The presets a profile offers, shipped ones when it names none.
#[cfg(feature = "election_config_archive")]
fn presets_for(
    profile: JsValue,
) -> Result<Vec<crate::election_config::preset_doc::AuthPresetDoc>, JsError> {
    let named: Vec<crate::election_config::preset_doc::AuthPresetDoc> =
        if profile.is_undefined() || profile.is_null() {
            Vec::new()
        } else {
            let document: profile::ClientProfile =
                serde_wasm_bindgen::from_value(profile).map_err(|error| {
                    JsError::new(&format!(
                        "this is not a client profile: {error}"
                    ))
                })?;
            document.auth_presets
        };

    if !named.is_empty() {
        return Ok(named);
    }

    profile::default_profile()
        .map(|shipped| shipped.auth_presets)
        .map_err(|error| {
            JsError::new(&format!(
                "the shipped profile could not be read: {error}"
            ))
        })
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
    let options = build_options(options)?;

    let read = match read_xlsx(workbook) {
        Ok(read) => read,
        Err(problem) => return failed(problem).map(IBuildOutput::from),
    };

    let templates = match TemplateSet::builtin() {
        Ok(templates) => templates,
        Err(problem) => return failed(problem).map(IBuildOutput::from),
    };

    let built = build(&read, &templates, &options, &Sources::default());

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
        delivery: None,
        report,
        event_external_id: Some(bundle.event_external_id.clone()),
    })
    .map(IBuildOutput::from)
}

/// Check a plan, in the wizard's own vocabulary.
///
/// A trustee threshold no number of trustees can meet, a key ceremony after
/// voting opens, a contest electing more people than are standing. Different
/// questions from [`check_bundle`], which asks about a built bundle in the
/// bundle's vocabulary — both run, because a problem phrased as
/// `contests[2].winning_candidates_num` is no use to somebody looking at a
/// wizard, and a bundle-level problem has no wizard field to point at.
#[cfg(feature = "election_config_archive")]
#[wasm_bindgen(js_name = validatePlan)]
pub fn validate_plan_js(plan: JsValue) -> Result<IReport, JsError> {
    let plan: architect::Blueprint = serde_wasm_bindgen::from_value(plan)
        .map_err(|error| {
            JsError::new(&format!("this is not an election plan: {error}"))
        })?;

    // Derived from the plan's own fields while the plan still carries them. The
    // additive `options.census` this grows in the next commit changes what is
    // passed here, not what this function is for.
    let sources = sources::Sources::from_plan(&plan);
    to_js(&architect::validate_plan(&plan, &sources)).map(IReport::from)
}

// -- the census, pulled from JavaScript ----------------------------------------

/// A census the host holds, read a batch at a time.
///
/// Declared as an extern type rather than deserialised, because a census is not
/// data here — it is three methods, and the whole point is that the rows never
/// cross the boundary as one value. Ten million members reaching Rust as a JS array
/// is the thing this exists to prevent, and it is also, exactly, what
/// `serde_wasm_bindgen::from_value` would do with them.
///
/// The shape is [`CensusCsvReader`]'s own, which is not a coincidence: that class
/// already exists, the wizard's census store already speaks to it, and a CSV the
/// browser has parsed once should not be parsed again to be handed over. A
/// `CensusCsvReader` **is** a `CensusPull`.
#[cfg(feature = "election_config_archive")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "CensusPull")]
    pub type CensusPull;

    /// The columns a row will have, in order. Answered before any row is read.
    #[wasm_bindgen(method, catch, js_name = columns)]
    fn columns(this: &CensusPull) -> Result<JsValue, JsValue>;

    /// Back to the first row. One compile reads the census more than once.
    #[wasm_bindgen(method, catch, js_name = rewind)]
    fn rewind(this: &CensusPull) -> Result<(), JsValue>;

    /// The next `size` rows as arrays of strings, aligned to `columns()`. An empty
    /// array means the census is done.
    #[wasm_bindgen(method, catch, js_name = nextBatch)]
    fn next_batch(this: &CensusPull, size: usize) -> Result<JsValue, JsValue>;

    /// The options object, reached for the two things serde cannot deserialise.
    #[wasm_bindgen(typescript_type = "CompileOptions")]
    pub type CompileOptions;

    #[wasm_bindgen(method, getter, js_name = census)]
    fn census(this: &CompileOptions) -> Option<CensusPull>;
}

/// A [`CensusPull`] as something the core can read.
#[cfg(feature = "election_config_archive")]
struct JsCensus {
    pull: CensusPull,
    shape: sources::RowShape,
    by_area_name: std::collections::BTreeMap<String, String>,
}

#[cfg(feature = "election_config_archive")]
impl JsCensus {
    /// **The columns are read here, once, and kept.**
    ///
    /// `CensusSource::columns` hands back a borrowed slice, so it cannot call into
    /// JavaScript; and it should not, because `build_realm::census_attributes` asks
    /// this question for every build and a round trip per call would be paid for
    /// nothing. Reading them at construction is also what makes the promise true —
    /// the column list is available before the first row.
    fn new(
        pull: CensusPull,
        by_area_name: std::collections::BTreeMap<String, String>,
    ) -> Result<Self, JsError> {
        let columns = pull.columns().map_err(|error| {
            JsError::new(&format!(
                "the census could not say what its columns are: {error:?}"
            ))
        })?;
        let columns: Vec<String> = serde_wasm_bindgen::from_value(columns)
            .map_err(|error| {
                JsError::new(&format!(
                    "a census column list is strings: {error}"
                ))
            })?;
        Ok(JsCensus {
            pull,
            shape: sources::RowShape::of(&columns),
            by_area_name,
        })
    }
}

#[cfg(feature = "election_config_archive")]
impl sources::CensusSource for JsCensus {
    fn columns(&self) -> &[String] {
        self.shape.columns()
    }

    fn rewind(&self) -> Result<(), String> {
        self.pull.rewind().map_err(|error| {
            format!("the census could not be reopened: {error:?}")
        })
    }

    fn next_batch(
        &self,
        size: usize,
    ) -> Result<Vec<architect::PlannedVoter>, String> {
        let batch = self.pull.next_batch(size).map_err(|error| {
            format!("the census could not be read past this point: {error:?}")
        })?;
        let rows: Vec<Vec<String>> = serde_wasm_bindgen::from_value(batch)
            .map_err(|error| {
                format!("a census batch is rows of strings: {error}")
            })?;
        Ok(rows
            .iter()
            .map(|row| self.shape.voter(row, &self.by_area_name))
            .collect())
    }
}

/// What the caller is handing over beside the plan, if anything.
///
/// **Additive on purpose.** A host that passes neither gets `None`, and
/// `compile_plan` then derives both from the plan's own fields exactly as before.
/// That is what lets this land before the browser half without a red window: beyond
/// builds against step's live tip, so a boundary that only grows is the only kind
/// that can be pushed first.
#[cfg(feature = "election_config_archive")]
fn sources_from(
    options: &JsValue,
    plan: &architect::Blueprint,
) -> Result<Option<sources::Sources>, JsError> {
    #[derive(serde::Deserialize, Default)]
    #[serde(default)]
    struct Carrying {
        /// Name to bytes. The plan names a file; this is the file.
        files: std::collections::BTreeMap<String, Vec<u8>>,
    }

    if !options.is_object() {
        return Ok(None);
    }

    // Reached through a getter rather than serde: a census is three methods, and
    // there is nothing here for `from_value` to deserialise.
    let census = options.unchecked_ref::<CompileOptions>().census();

    let carrying: Carrying = serde_wasm_bindgen::from_value(options.clone())
        .map_err(|error| JsError::new(&format!("bad options: {error}")))?;

    if census.is_none() && carrying.files.is_empty() {
        return Ok(None);
    }

    // A name is what a plan points at, so the areas have to be resolvable before a
    // row is read: a census that says `area_name` is matched back to the
    // `external_id` the plan keys by.
    let by_area_name = plan
        .areas
        .iter()
        .map(|area| (area.name.clone(), area.external_id.clone()))
        .collect();

    Ok(Some(sources::Sources {
        census: match census {
            Some(pull) => {
                Some(std::sync::Arc::new(JsCensus::new(pull, by_area_name)?))
            }
            None => None,
        },
        files: carrying
            .files
            .into_iter()
            .map(|(name, bytes)| (name, std::sync::Arc::from(bytes)))
            .collect(),
    }))
}

/// Compile a plan into the bundle and the files that travel beside it.
///
/// The same [`Output`] `buildFromWorkbook` returns, because a wizard and a
/// spreadsheet produce the same thing and a front end should not need two shapes
/// to hold it.
///
/// A bad plan comes back as a report with no files rather than as an exception:
/// a list of problems is something a page can render. An exception here means
/// the core itself failed, which is a different thing and belongs somewhere
/// else in the interface.
#[cfg(feature = "election_config_archive")]
#[wasm_bindgen(js_name = compilePlan)]
pub fn compile_plan_js(
    plan: JsValue,
    options: JsValue,
) -> Result<IBuildOutput, JsError> {
    let plan: architect::Blueprint = serde_wasm_bindgen::from_value(plan)
        .map_err(|error| {
            JsError::new(&format!("this is not an election plan: {error}"))
        })?;

    // The profile travels *inside* options rather than as a third argument.
    // It was a third argument, and the browser passed two — so every profile
    // silently became `None`, `apply_profile` never ran, and not one locked
    // value reached a bundle. Nothing on either side complained, because a
    // missing positional at an FFI boundary is `undefined` and `undefined`
    // deserializes to `Option::None`. A named field cannot be dropped that way.
    let profile = profile_from(&options)?;
    let sources = sources_from(&options, &plan)?;
    let options = build_options(options)?;

    let templates = match TemplateSet::builtin() {
        Ok(templates) => templates,
        Err(problem) => return failed(problem).map(IBuildOutput::from),
    };

    let compiled = match architect::compile_plan(architect::Compile {
        plan: &plan,
        templates: &templates,
        options: &options,
        profile: profile.as_ref(),
        sources: sources.as_ref(),
    }) {
        Ok(compiled) => compiled,
        Err(report) => {
            return to_js(&Output::refused(report)).map(IBuildOutput::from)
        }
    };

    let zipped = match archive::zip(&compiled.layout.importable) {
        Ok(bytes) => bytes,
        Err(problem) => return failed(problem).map(IBuildOutput::from),
    };

    to_js(&Output {
        importable: compiled.layout.importable.iter().map(File::from).collect(),
        auxiliary: compiled.layout.auxiliary.iter().map(File::from).collect(),
        archive: Some(File {
            name: compiled.layout.archive_name.clone(),
            bytes: zipped,
        }),
        delivery: match archive::delivery(&compiled.layout) {
            Ok(artifact) => Some(File::from(&artifact)),
            // A delivery that cannot be written is a bug in this tool rather than a
            // problem with the plan, and the importable zip above is unaffected — so
            // the build still succeeds and the host falls back to it.
            Err(_) => None,
        },
        report: compiled.report,
        event_external_id: Some(compiled.bundle.event_external_id.clone()),
    })
    .map(IBuildOutput::from)
}

/// The ballots this plan's voters would be given.
///
/// Compiles the plan the way [`compile_plan_js`] does — profile applied,
/// validated, built — and then hands the resulting entities to the platform's own
/// ballot-style builder. So the preview is generated from the bytes that would be
/// imported, and nothing in this crate decides what a ballot contains.
///
/// No zip. A review screen asking "what will voters see?" does not need the
/// archive, and compressing one to answer would be a second of a delivery
/// engineer's time on every keystroke.
///
/// Like the other entry points, a plan that cannot be compiled comes back as a
/// report with no preview rather than as an exception.
#[cfg(feature = "election_config_archive")]
#[wasm_bindgen(js_name = previewBallot)]
pub fn preview_ballot_js(
    plan: JsValue,
    options: JsValue,
) -> Result<IPreviewOutput, JsError> {
    let plan: architect::Blueprint = serde_wasm_bindgen::from_value(plan)
        .map_err(|error| {
            JsError::new(&format!("this is not an election plan: {error}"))
        })?;

    let profile = profile_from(&options)?;
    let demo_key = demo_public_key_from(&options)?;
    let sources = sources_from(&options, &plan)?;
    let options = build_options(options)?;

    let templates = match TemplateSet::builtin() {
        Ok(templates) => templates,
        Err(problem) => {
            return refused_preview(problem).map(IPreviewOutput::from)
        }
    };

    // The same call `compile_plan` makes, so a plan that previews is a plan that
    // compiles and vice versa. Anything else and the review screen would show a
    // ballot for a bundle that will not build.
    let compiled = match architect::compile_plan(architect::Compile {
        plan: &plan,
        templates: &templates,
        options: &options,
        profile: profile.as_ref(),
        sources: sources.as_ref(),
    }) {
        Ok(compiled) => compiled,
        Err(report) => {
            return to_js(&PreviewOutput {
                preview: None,
                areas: Vec::new(),
                elections: Vec::new(),
                report,
            })
            .map(IPreviewOutput::from)
        }
    };

    let settings = preview::PreviewOptions {
        demo_public_key: demo_key
            .unwrap_or_else(|| preview::NOT_A_KEY.to_string()),
    };

    match preview::preview_publication(&compiled.bundle, &settings) {
        Ok(document) => {
            // The schema the ballots were generated from, re-read only to put
            // names on the areas. It cannot fail here: `preview_publication`
            // just parsed the same value.
            // The schema is re-read once and both pickers are named from it.
            let schema: Option<_> =
                serde_json::from_value(compiled.bundle.export.clone()).ok();
            let areas = schema
                .as_ref()
                .map(|schema| document.areas(schema))
                .unwrap_or_default();
            let elections = schema
                .as_ref()
                .map(|schema| document.elections(schema))
                .unwrap_or_default();
            to_js(&PreviewOutput {
                preview: Some(document.to_document()),
                areas,
                elections,
                report: compiled.report,
            })
            .map(IPreviewOutput::from)
        }
        Err(mut report) => {
            for problem in compiled.report.problems {
                report.push(problem);
            }
            to_js(&PreviewOutput {
                preview: None,
                areas: Vec::new(),
                elections: Vec::new(),
                report,
            })
            .map(IPreviewOutput::from)
        }
    }
}

/// What a preview shows in place of the key no ceremony has produced yet.
///
/// Optional, and normally absent. It is here for the delivery engineer previewing
/// against a tenant whose ceremony *has* run, who wants the real thing rather
/// than a stand-in.
#[cfg(feature = "election_config_archive")]
fn demo_public_key_from(options: &JsValue) -> Result<Option<String>, JsError> {
    #[derive(serde::Deserialize, Default)]
    #[serde(default, rename_all = "camelCase")]
    struct Carrier {
        demo_public_key: Option<String>,
    }

    if options.is_undefined() || options.is_null() {
        return Ok(None);
    }
    let carrier: Carrier = serde_wasm_bindgen::from_value(options.clone())
        .map_err(|error| JsError::new(&format!("bad options: {error}")))?;
    Ok(carrier.demo_public_key)
}

/// One problem, as a preview with nothing in it.
#[cfg(feature = "election_config_archive")]
fn refused_preview(problem: Problem) -> Result<JsValue, JsError> {
    let mut report = Report::default();
    report.push(problem);
    to_js(&PreviewOutput {
        preview: None,
        areas: Vec::new(),
        elections: Vec::new(),
        report,
    })
}

#[cfg(feature = "election_config_archive")]
#[derive(Serialize)]
struct PreviewOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    preview: Option<serde_json::Value>,
    areas: Vec<preview::PreviewArea>,
    elections: Vec<preview::PreviewArea>,
    report: Report,
}

/// The client profile out of the options object, if it carries one.
///
/// Returns an error rather than a report: a malformed profile is a broken
/// deployment, not a plan somebody can fix by editing their answers, and
/// swallowing it into the build report is how it goes unnoticed.
#[cfg(feature = "election_config_archive")]
fn profile_from(
    options: &JsValue,
) -> Result<Option<profile::Profile>, JsError> {
    #[derive(serde::Deserialize, Default)]
    #[serde(default)]
    struct Carrying {
        profile: Option<profile::ClientProfile>,
    }

    if options.is_undefined() || options.is_null() {
        return Ok(None);
    }

    let carrying: Carrying = serde_wasm_bindgen::from_value(options.clone())
        .map_err(|error| JsError::new(&format!("bad options: {error}")))?;

    let Some(document) = carrying.profile else {
        return Ok(None);
    };

    profile::Profile::read(&document)
        .map(Some)
        .map_err(|report| {
            JsError::new(&format!(
                "this client profile cannot be used:\n{report}"
            ))
        })
}

/// Read a client profile, or none.
///
/// A malformed profile comes back as a report rather than an exception, for the
/// same reason a malformed plan does: it is a document somebody wrote, and its
/// problems are a list a page can render.
#[cfg(feature = "election_config_archive")]
fn read_profile(profile: JsValue) -> Result<Option<profile::Profile>, Report> {
    if profile.is_undefined() || profile.is_null() {
        return Ok(None);
    }

    let document: profile::ClientProfile =
        serde_wasm_bindgen::from_value(profile).map_err(|error| {
            let mut report = Report::default();
            report.push(Problem::error(
                Code::InvalidValue,
                "profile",
                format!("this is not a client profile: {error}"),
            ));
            report
        })?;

    profile::Profile::read(&document).map(Some)
}

/// The plan inside a delivery zip, so `Import Configuration` can open what a client kept.
///
/// A client keeps the whole delivery, not the `blueprint.json` inside it — so the wizard
/// has to open the zip rather than asking somebody to unzip it and pick the right file out
/// of eight, one of which can carry administrator passwords.
///
/// Returns the plan as a `JsValue`, already parsed, because the host's next move is to put
/// it into wizard state. A zip with no plan in it comes back as an error naming what the
/// zip *did* contain, which is the difference between fixing it and guessing.
#[cfg(feature = "election_config_archive")]
#[wasm_bindgen(js_name = planInDelivery)]
pub fn plan_in_delivery_js(bytes: &[u8]) -> Result<JsValue, JsError> {
    let plan = archive::plan_in_delivery(bytes)
        .map_err(|problem| JsError::new(&problem.message))?;

    let value: serde_json::Value =
        serde_json::from_slice(&plan).map_err(|error| {
            JsError::new(&format!(
                "the plan inside this configuration is not readable: {error}"
            ))
        })?;

    to_js(&value)
}

/// A delivery zip, a plan `.json` or a workbook `.xlsx` — whichever this is.
///
/// One call for all three, because telling them apart cannot be done outside Rust:
/// an `.xlsx` has the same `PK` magic as a delivery, so a host sniffing bytes hands
/// a spreadsheet to the delivery reader and gets "no plan in this zip" about a
/// perfectly good workbook. Opening the archive and looking inside is the archive
/// reader's job.
///
/// **A bad file comes back as a report rather than as an exception**, which is the
/// rule the rest of this module follows and the reason it matters here: a report is
/// something a screen can group by tab and point at cells with, and an exception is
/// a string. `plan` is `null` when nothing could be read.
#[cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_archive"
))]
#[wasm_bindgen(js_name = openConfiguration)]
pub fn open_configuration(
    bytes: &[u8],
    // The file's own name, for the one refusal that cannot be read out of the
    // bytes: an encrypted `.ezip` is AES-CBC from its first byte, so it has no
    // zip magic and no valid UTF-8, and without the name it comes back as
    // "neither a .zip, an .xlsx, nor text". Optional, so every existing caller
    // keeps working and a renamed file still opens as whatever it is.
    name: Option<String>,
) -> Result<JsValue, JsError> {
    #[derive(serde::Serialize)]
    struct Answer {
        plan: Option<serde_json::Value>,
        report: Report,
        source: Option<crate::election_config::open::Source>,
    }

    let answer = match crate::election_config::open::open_named(
        bytes,
        name.as_deref(),
    ) {
        Ok(opened) => Answer {
            plan: Some(serde_json::to_value(&opened.plan).map_err(
                |error| {
                    JsError::new(&format!(
                        "this plan could not be read: {error}"
                    ))
                },
            )?),
            report: opened.report,
            source: Some(opened.source),
        },
        Err(report) => Answer {
            plan: None,
            report,
            source: None,
        },
    };

    to_js(&answer)
}

// -- opening, with what travelled beside the plan ------------------------------

/// A census the core holds, handed out under the interface it takes in.
///
/// **The same three methods as `CensusPull`, pointing the other way.** A host that
/// opens a save file gets one of these and can pass it straight back to
/// `compilePlan` as `options.census`, so ten million members are read from the zip,
/// counted, and written into a bundle without ever being a JavaScript value.
///
/// `CensusCsvReader` is the same shape and stays, because a dropped CSV has no
/// `Opened` to come from. The two are interchangeable wherever a `CensusPull` is
/// wanted, which is the whole reason the interface was written before either.
#[cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_archive"
))]
#[wasm_bindgen(js_name = CensusHandle)]
pub struct CensusHandle {
    inner: std::sync::Arc<dyn sources::CensusSource>,
}

#[cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_archive"
))]
#[wasm_bindgen(js_class = CensusHandle)]
impl CensusHandle {
    /// The columns a row will have, in order.
    #[wasm_bindgen(js_name = columns)]
    pub fn columns(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(self.inner.columns())
            .map_err(JsError::from)
    }

    /// Back to the first row.
    #[wasm_bindgen(js_name = rewind)]
    pub fn rewind(&self) -> Result<(), JsError> {
        self.inner.rewind().map_err(|why| JsError::new(&why))
    }

    /// The next `size` rows as arrays of strings, aligned to `columns()`.
    #[wasm_bindgen(js_name = nextBatch)]
    pub fn next_batch(&self, size: usize) -> Result<JsValue, JsError> {
        let batch = self
            .inner
            .next_batch(size)
            .map_err(|why| JsError::new(&why))?;
        let columns = self.inner.columns();
        let rows: Vec<Vec<&str>> = batch
            .iter()
            .map(|voter| {
                columns
                    .iter()
                    .map(|column| sources::cell_of(voter, column))
                    .collect()
            })
            .collect();
        serde_wasm_bindgen::to_value(&rows).map_err(JsError::from)
    }
}

/// What `openFile` made of the bytes.
///
/// A struct rather than a plain object because two of its members must not be
/// serialised: the census is a handle, and the files are bytes a host wants as
/// `Uint8Array`s rather than as arrays of numbers.
#[cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_archive"
))]
#[wasm_bindgen(js_name = OpenedConfiguration)]
pub struct OpenedConfiguration {
    plan: Option<serde_json::Value>,
    report: Report,
    source: Option<crate::election_config::open::Source>,
    sources: Option<sources::Sources>,
}

#[cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_archive"
))]
#[wasm_bindgen(js_class = OpenedConfiguration)]
impl OpenedConfiguration {
    /// The plan, or `null` when nothing could be read. Read `report` then.
    #[wasm_bindgen(js_name = plan)]
    pub fn plan(&self) -> Result<JsValue, JsError> {
        to_js(&self.plan)
    }

    /// Everything found on the way, errors and warnings together.
    #[wasm_bindgen(js_name = report)]
    pub fn report(&self) -> Result<JsValue, JsError> {
        to_js(&self.report)
    }

    /// What the bytes turned out to be: `delivery`, `plan-archive`, `workbook`…
    #[wasm_bindgen(js_name = source)]
    pub fn source(&self) -> Result<JsValue, JsError> {
        to_js(&self.source)
    }

    /// The census that travelled with it, if one did.
    ///
    /// **Taken, not borrowed.** A handle owns its cursor, and two of them over one
    /// census would each think they were at the start. Calling this twice returns
    /// `undefined` the second time rather than a second reader of the same rows.
    #[wasm_bindgen(js_name = takeCensus)]
    pub fn take_census(&mut self) -> Option<CensusHandle> {
        self.sources
            .as_mut()
            .and_then(|sources| sources.census.take())
            .map(|inner| CensusHandle { inner })
    }

    /// The bytes the plan's file names point at, as `Record<string, Uint8Array>`.
    ///
    /// Serialised in one go, unlike the census, because these are a logo and some
    /// photographs — kilobytes each, and a host wants them as values to put in
    /// state rather than as a stream to pull.
    #[wasm_bindgen(js_name = files)]
    pub fn files(&self) -> Result<JsValue, JsError> {
        let files: std::collections::BTreeMap<&str, &[u8]> = self
            .sources
            .as_ref()
            .map(|sources| {
                sources
                    .files
                    .iter()
                    .map(|(name, bytes)| (name.as_str(), bytes.as_ref()))
                    .collect()
            })
            .unwrap_or_default();
        serde_wasm_bindgen::to_value(&files).map_err(JsError::from)
    }
}

/// Open a delivery, a save file, a plan, a workbook or an event export.
///
/// The successor to [`open_configuration`], which stays for now because beyond's
/// CI builds this crate's live tip and a boundary that only grows is the only kind
/// that can be pushed first. The difference is everything the older one had no
/// place for: a save file's census and the files it carries.
#[cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_archive"
))]
#[wasm_bindgen(js_name = openFile)]
pub fn open_file(
    bytes: &[u8],
    name: Option<String>,
) -> Result<OpenedConfiguration, JsError> {
    Ok(
        match crate::election_config::open::open_named(bytes, name.as_deref()) {
            Ok(opened) => OpenedConfiguration {
                plan: Some(serde_json::to_value(&opened.plan).map_err(
                    |error| {
                        JsError::new(&format!(
                            "this plan could not be read: {error}"
                        ))
                    },
                )?),
                report: opened.report,
                source: Some(opened.source),
                sources: Some(opened.sources),
            },
            Err(report) => OpenedConfiguration {
                plan: None,
                report,
                source: None,
                sources: None,
            },
        },
    )
}

/// The zip the wizard hands over when somebody saves.
///
/// **Never a bare `blueprint.json`.** A plan on its own is a plan with the members'
/// names and the candidates' photographs missing, so what comes back is always an
/// archive: the plan, its census and the files it names.
#[cfg(all(
    feature = "election_config_xlsx",
    feature = "election_config_archive"
))]
#[wasm_bindgen(js_name = saveFile)]
pub fn save_file_js(
    plan: JsValue,
    options: JsValue,
) -> Result<JsValue, JsError> {
    let plan: architect::Blueprint = serde_wasm_bindgen::from_value(plan)
        .map_err(|error| {
            JsError::new(&format!("this is not an election plan: {error}"))
        })?;

    let derived;
    let sources = match sources_from(&options, &plan)? {
        Some(sources) => {
            derived = sources;
            &derived
        }
        None => {
            derived = sources::Sources::from_plan(&plan);
            &derived
        }
    };

    let artifact = archive::save_file(&plan, sources)
        .map_err(|problem| JsError::new(&problem.message))?;
    to_js(&File::from(&artifact))
}

/// What a profile hides, so the wizard knows what not to draw.
///
/// Rust decides which *paths*, because that is a statement about the plan.
/// Which screens that empties is a question about screens, and belongs to
/// whoever draws them.
/// A plan with the profile's own values already in it.
///
/// The wizard could not do this, and the hole was visible: a delivery engineer
/// fixes "SMART Elections" as the event name, opens the client's link, and the
/// field is empty. The value was real — `apply_profile` runs inside
/// `compile_plan`, so the *built* archive had it — but nothing put it on the
/// screen, so the client saw a blank box above an error telling them to fill it
/// in.
///
/// Exposed rather than reimplemented in TypeScript. The rules are not obvious —
/// a **fixed** path (locked or hidden) is written unconditionally, while any
/// other default is written only where the plan says nothing, so seeding cannot
/// overwrite an answer somebody already gave — and a second copy of that in
/// another language is a second opinion about what a profile means.
#[cfg(feature = "election_config_archive")]
#[wasm_bindgen(js_name = applyProfile)]
pub fn apply_profile_js(
    plan: JsValue,
    profile: JsValue,
) -> Result<JsValue, JsError> {
    let plan: architect::Blueprint = serde_wasm_bindgen::from_value(plan)
        .map_err(|error| JsError::new(&format!("bad plan: {error}")))?;
    let document: profile::ClientProfile =
        serde_wasm_bindgen::from_value(profile)
            .map_err(|error| JsError::new(&format!("bad profile: {error}")))?;
    let read = profile::Profile::read(&document).map_err(|report| {
        JsError::new(&format!(
            "this profile cannot be read: {}",
            report
                .problems
                .first()
                .map(|problem| problem.message.clone())
                .unwrap_or_default()
        ))
    })?;
    let seeded = profile::apply_profile(&plan, &read).map_err(|report| {
        JsError::new(&format!(
            "this profile cannot be applied: {}",
            report
                .problems
                .first()
                .map(|problem| problem.message.clone())
                .unwrap_or_default()
        ))
    })?;
    // `Serializer::json_compatible`, like every other export here: the default
    // renders a map as a `Map`, and a front end reads properties.
    seeded
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|error| JsError::new(&format!("cannot hand back: {error}")))
}

#[cfg(feature = "election_config_archive")]
#[wasm_bindgen(js_name = readProfile)]
pub fn read_profile_js(profile: JsValue) -> Result<JsValue, JsError> {
    #[derive(Serialize)]
    struct Read {
        id: String,
        display_name: Option<String>,
        hidden: Vec<String>,
        locked: Vec<String>,
        /// Handed over so a front end can mark a field, even though enforcing
        /// it is Rust's job — a required field with no asterisk is a form
        /// somebody fills in twice.
        required: Vec<String>,
        /// The ballot-rule sets this client is offered, already checked against
        /// the real value space. A front end renders these as buttons without
        /// having to know what a policy is, which is what stops one offering a
        /// behaviour no importer accepts.
        presets: Vec<profile::NamedPreset>,
        /// Whether ours are offered alongside.
        only_our_presets: bool,
        /// Whether the ballot preview starts without the portal's chrome. A
        /// preference about the wizard rather than about the election, which is
        /// why it travels here and not through `defaults`.
        preview_slim: bool,
        warnings: Report,
    }

    let document: profile::ClientProfile =
        serde_wasm_bindgen::from_value(profile).map_err(|error| {
            JsError::new(&format!("this is not a client profile: {error}"))
        })?;

    match profile::Profile::read(&document) {
        Ok(read) => to_js(&Read {
            id: read.id.clone(),
            display_name: read.display_name.clone(),
            hidden: read
                .hidden_paths()
                .into_iter()
                .map(str::to_string)
                .collect(),
            presets: read.presets.clone(),
            only_our_presets: read.only_our_presets,
            preview_slim: read.preview_slim,
            locked: read
                .locked_paths()
                .into_iter()
                .map(str::to_string)
                .collect(),
            required: read
                .required_paths()
                .into_iter()
                .map(str::to_string)
                .collect(),
            warnings: read.warnings.clone(),
        }),
        // An error, not a `Report` returned as success. The caller casts this
        // result to a profile; handing back `{problems: [...]}` produced
        // `profile.hidden` undefined and a blank page, with the very problems
        // Rust took care to produce thrown away.
        Err(report) => Err(JsError::new(&format!(
            "this client profile cannot be used:\n{report}"
        ))),
    }
}

/// The ballot-behaviour values a picker may offer.
///
/// Returned rather than duplicated in TypeScript, for the same reason
/// [`auth_presets`] is: a dropdown cannot list a value the platform does not
/// have, and cannot miss one it does. `labels` is the namespace the Admin
/// Portal's translations are keyed by, so a front end labels a value without a
/// second table of its own.
///
/// This is the third place the value space appears — `ContestPresentation.ts`
/// and `contest.hbs` being the others — and handing it over as data is what
/// stops a fourth.
#[cfg(feature = "election_config_archive")]
#[wasm_bindgen(js_name = policyCatalog)]
pub fn policy_catalog() -> Result<JsValue, JsError> {
    use crate::election_config::policy::{
        BlankVote, CandidatesOrder, DuplicatedRank, InvalidVote, OverVote,
        Policies, PolicyValue, PreferenceGaps, UnderVote,
    };
    use crate::types::ceremonies::CeremoniesPolicy;
    use strum::IntoEnumIterator;

    #[derive(Serialize)]
    struct Kind {
        /// The plan field this sets, e.g. `over_vote`.
        field: &'static str,
        /// The bundle column it becomes.
        column: &'static str,
        /// The Admin Portal translation namespace.
        labels: &'static str,
        /// Every value, most permissive first.
        values: Vec<&'static str>,
        /// What a plan gets when it says nothing.
        default: &'static str,
    }

    fn kind<T: PolicyValue + IntoEnumIterator + Default>(
        field: &'static str,
    ) -> Kind {
        Kind {
            field,
            column: T::COLUMN,
            labels: T::LABELS,
            values: T::iter().map(PolicyValue::as_str).collect(),
            default: T::default().as_str(),
        }
    }

    /// How a contest is counted, which is not a policy but is set beside them.
    ///
    /// Handed over for the same reason the policies are: so a screen cannot
    /// offer `alphabetic` where the platform says `alphabetical`, or a
    /// preferential contest counted by a plurality algorithm. The two lists of
    /// algorithms are the real ones — `COUNTING_ALGORITHMS` and the subset
    /// `PREFERENTIAL_ALGORITHMS` — so a front end can refuse the combination
    /// rather than reporting it after the fact.
    #[derive(Serialize)]
    struct TallyCatalog {
        voting_types: &'static [&'static str],
        counting_algorithms: &'static [&'static str],
        /// The subset a dropdown should offer.
        ///
        /// Both lists cross, because they answer different questions: a *validator*
        /// asks whether a value is real, and a *picker* asks what somebody should be
        /// choosing between. Four of the ten are one municipal family whose names a
        /// label cannot distinguish, so offering them is offering four ways to get
        /// it wrong — but a plan that already names one still validates, still
        /// builds, and is still shown by a front end that keeps the current value
        /// in its list.
        offered_counting_algorithms: &'static [&'static str],
        /// Which of those are ranked. A contest using one must be preferential.
        preferential: &'static [&'static str],
        default_voting_type: String,
        default_counting_algorithm: String,
        default_min_votes: i64,
        default_is_encrypted: bool,
        tie_breaking_policies: &'static [&'static str],
        default_tie_breaking_policy: String,
    }

    /// How a contest's options are arranged on the page.
    ///
    /// Its own block rather than more `kinds`, because `kinds` is a list of
    /// enums with a shared shape — a column, a label namespace, a set of string
    /// values — and two of these are numbers. A front end draws them from here
    /// and cannot invent a value the platform lacks, which is the only property
    /// that matters.
    #[derive(Serialize)]
    struct LayoutCatalog {
        collapsible_lists: &'static [&'static str],
        enable_checkable_lists: &'static [&'static str],
        default_columns: i64,
        default_collapsible_lists: String,
        default_enable_checkable_lists: String,
        default_max_selections_per_type: i64,
    }

    /// What the Election Event screen may offer, from the platform rather than
    /// from a list somebody keeps in step by hand (`INV-8`).
    #[derive(Serialize)]
    struct EventCatalog {
        language_detection_policies: &'static [&'static str],
        default_language_detection_policy: String,
    }

    #[derive(Serialize)]
    struct Catalog {
        kinds: Vec<Kind>,
        event: EventCatalog,
        /// Named sets, so a wizard can offer a decision rather than seven.
        presets: Vec<(&'static str, Policies)>,
        tally: TallyCatalog,
        layout: LayoutCatalog,
        /// Who runs the key ceremony, and what a plan gets by saying nothing.
        ///
        /// Handed over rather than restated in TypeScript for the same reason as
        /// everything above it. The default matters as much as the values here:
        /// absent from a ceremony's `settings`, `KeysCeremony::policy()` reads
        /// `manual-ceremonies`, so a wizard that offered the choice without
        /// knowing the fallback could show the wrong one as already selected.
        ceremony: CeremonyCatalog,
    }

    #[derive(Serialize)]
    struct CeremonyCatalog {
        values: Vec<&'static str>,
        default: &'static str,
    }

    let ceremony = CeremonyCatalog {
        values: CeremoniesPolicy::iter()
            .map(|each| match each {
                CeremoniesPolicy::MANUAL_CEREMONIES => "manual-ceremonies",
                CeremoniesPolicy::AUTOMATED_CEREMONIES => {
                    "automated-ceremonies"
                }
            })
            .collect(),
        default: "manual-ceremonies",
    };

    to_js(&Catalog {
        event: EventCatalog {
            language_detection_policies:
                crate::election_config::validate::LANGUAGE_DETECTION_POLICIES,
            // `LanguageDetectionPolicy`'s own `#[default]`.
            default_language_detection_policy: "browser-detect".to_string(),
        },
        kinds: vec![
            kind::<OverVote>("over_vote"),
            kind::<BlankVote>("blank_vote"),
            kind::<UnderVote>("under_vote"),
            kind::<InvalidVote>("invalid_vote"),
            kind::<DuplicatedRank>("duplicated_rank"),
            kind::<PreferenceGaps>("preference_gaps"),
            kind::<CandidatesOrder>("candidates_order"),
        ],
        presets: vec![
            ("permissive", Policies::permissive()),
            ("standard", Policies::standard()),
            ("strict", Policies::strict()),
        ],
        tally: {
            let default = crate::election_config::policy::Tally::default();
            TallyCatalog {
                voting_types: crate::election_config::validate::VOTING_TYPES,
                counting_algorithms:
                    crate::election_config::validate::COUNTING_ALGORITHMS,
                offered_counting_algorithms:
                    crate::election_config::validate::OFFERED_COUNTING_ALGORITHMS,
                preferential:
                    crate::election_config::validate::PREFERENTIAL_ALGORITHMS,
                default_voting_type: default.voting_type.clone(),
                default_counting_algorithm: default.counting_algorithm.clone(),
                default_min_votes: default.min_votes,
                default_is_encrypted: default.is_encrypted,
                tie_breaking_policies:
                    crate::election_config::validate::TIE_BREAKING_POLICIES,
                default_tie_breaking_policy: default
                    .tie_breaking_policy
                    .clone(),
            }
        },
        layout: {
            let default = crate::election_config::policy::Layout::default();
            LayoutCatalog {
                collapsible_lists:
                    crate::election_config::validate::COLLAPSIBLE_LISTS,
                enable_checkable_lists:
                    crate::election_config::validate::CHECKABLE_LISTS,
                default_columns: default.columns,
                default_collapsible_lists: default.collapsible_lists.clone(),
                default_enable_checkable_lists: default
                    .enable_checkable_lists
                    .clone(),
                default_max_selections_per_type: default
                    .max_selections_per_type,
            }
        },
        ceremony,
    })
}

/// The options both entry points take, read once.
///
/// Declared here rather than inside each function so the two cannot drift the
/// first time [`BuildOptions`] gains a field.
#[cfg(feature = "election_config_archive")]
fn build_options(options: JsValue) -> Result<BuildOptions, JsError> {
    // `serde_wasm_bindgen` ignores unknown keys, so the `profile` the plan path
    // also puts in here passes straight through.
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

    Ok(BuildOptions {
        // Empty here and filled by `compile_plan`, which derives them from the plan
        // — the same as `images`. A caller cannot pass bytes through this boundary
        // as options, and would not want to: the plan already holds them.
        materials: Vec::new(),
        tenant_id: options.tenant_id,
        base_export: options.base_export,
        slug: options.slug,
        created_at: options.created_at,
        auth_preset: options.auth_preset,
        // Not from JavaScript: `compile_plan` derives it from the plan's own
        // trustees, and `build_from_workbook` has no trustees to derive it from.
        keys_ceremony: None,
        // Same again. The photographs are in the plan, `compile_plan` walks them out
        // with `plan_images`, and the workbook path has none — a spreadsheet cell
        // cannot hold an image. Handing them across the boundary as base64 a second
        // time would mean two ways to say the same thing and one of them stale.
        images: Vec::new(),
        ceremony_policy: Default::default(),
    })
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

    /// The whole delivery: one zip that is **not** importable, holding one that is.
    ///
    /// `archive` above is the importable member on its own, and it stays because the
    /// downloads panel names it as the one file the Admin Portal takes. This is what a
    /// client is actually handed — `official_election_setup.zip` nested beside the
    /// reopenable plan, the points of contact, the trustee list and the ceremony dates,
    /// in `election_architect`'s own layout.
    ///
    /// Only the plan path produces one. A workbook build is already a bundle rather than
    /// a delivery, and its caller hands the importable zip over directly.
    delivery: Option<File>,
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
            delivery: None,
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
///
/// `serialize_maps_as_objects`, which is not a preference. `serde_wasm_bindgen`
/// renders anything serialised through `serialize_map` — a `HashMap`, and every
/// `serde_json::Value::Object` — as a JS **`Map`**, whose members are not
/// properties. `preview_ballot` returns a `serde_json::Value` on purpose (a
/// `Value`'s object is a `BTreeMap`, so the document's keys come out sorted and
/// two previews of one plan diff cleanly), and the review screen read
/// `document.ballot_styles` off it: `undefined`, and `undefined.filter` took the
/// whole wizard down to a white screen. `JSON.stringify` shows a `Map` as `{}`,
/// so it looked like an empty document rather than a wrong one.
///
/// Not `Serializer::json_compatible()`, though it is what `wasm/areas.rs` uses:
/// that also turns `None` into `null` instead of `undefined` and changes how
/// bytes serialise, and `compile_plan` hands artifacts across as bytes. One flag,
/// for the one thing that was wrong.
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    let serializer =
        serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value
        .serialize(&serializer)
        .map_err(|error| JsError::new(&format!("could not convert: {error}")))
}

/// A census CSV, read a batch at a time.
///
/// A handle rather than a function returning every voter, and that is the whole
/// point: a hundred thousand members crossing the boundary at once is the copy
/// this exists to avoid. The caller asks for a batch, turns it into whatever it
/// stores, and drops it before asking for the next.
///
/// The wizard used to parse this in TypeScript. Two parsers meant two answers to
/// "what is in this file", and on a census that is how a quoted comma read one way
/// by the loader and another by the saver turns one member into two on the way
/// out.
#[wasm_bindgen(js_name = CensusCsvReader)]
pub struct CensusCsvReader {
    inner: census_csv::CensusCsv,
}

#[wasm_bindgen(js_class = CensusCsvReader)]
impl CensusCsvReader {
    /// Read the header. Throws on a file with nothing in it or no `username`.
    #[wasm_bindgen(constructor)]
    pub fn new(text: &str) -> Result<CensusCsvReader, JsError> {
        census_csv::CensusCsv::new(text)
            .map(|inner| CensusCsvReader { inner })
            .map_err(|message| JsError::new(&message))
    }

    /// The columns a voter will have, in order, with the derived ones dropped.
    #[wasm_bindgen(js_name = columns)]
    pub fn columns(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(&self.inner.header().columns)
            .map_err(JsError::from)
    }

    /// What was odd about the file without being wrong with it.
    #[wasm_bindgen(js_name = notes)]
    pub fn notes(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(&self.inner.header().notes)
            .map_err(JsError::from)
    }

    /// The next `size` rows as arrays of strings. Empty when the file is done.
    #[wasm_bindgen(js_name = nextBatch)]
    pub fn next_batch(&mut self, size: usize) -> Result<JsValue, JsError> {
        let batch = self
            .inner
            .next_batch(size)
            .map_err(|message| JsError::new(&message))?;
        serde_wasm_bindgen::to_value(&batch).map_err(JsError::from)
    }
}
