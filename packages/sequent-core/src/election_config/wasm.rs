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

use crate::election_config::fixtures;
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
}

export interface Report {
    problems: Problem[];
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
    /** Everything found on the way, errors and warnings together. */
    report: Report;
}
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
        /// User-profile attributes this preset reads off a voter.
        ///
        /// The census's column chooser offers these, so a column somebody adds is one
        /// the sign-in flow can actually read rather than one Keycloak drops.
        profile_attributes: Vec<&'static str>,
    }

    let listed: Vec<Listed> = presets::PRESETS
        .iter()
        .map(|preset| Listed {
            name: preset.name,
            summary: preset.summary,
            uses_otp: preset.uses_otp,
            required_parameters: preset.required_parameters.to_vec(),
            optional_parameters: preset.optional_parameters.to_vec(),
            profile_attributes: preset.profile_attributes.to_vec(),
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
    let options = build_options(options)?;

    let read = match read_xlsx(workbook) {
        Ok(read) => read,
        Err(problem) => return failed(problem).map(IBuildOutput::from),
    };

    let templates = match TemplateSet::builtin() {
        Ok(templates) => templates,
        Err(problem) => return failed(problem).map(IBuildOutput::from),
    };

    let built = build(&read, &templates, &options);

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

    to_js(&architect::validate_plan(&plan)).map(IReport::from)
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
    let options = build_options(options)?;

    let templates = match TemplateSet::builtin() {
        Ok(templates) => templates,
        Err(problem) => return failed(problem).map(IBuildOutput::from),
    };

    let compiled = match architect::compile_plan(
        &plan,
        &templates,
        &options,
        profile.as_ref(),
    ) {
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
    let compiled = match architect::compile_plan(
        &plan,
        &templates,
        &options,
        profile.as_ref(),
    ) {
        Ok(compiled) => compiled,
        Err(report) => {
            return to_js(&PreviewOutput {
                preview: None,
                areas: Vec::new(),
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
            let areas = serde_json::from_value(compiled.bundle.export.clone())
                .map(|schema| document.areas(&schema))
                .unwrap_or_default();
            to_js(&PreviewOutput {
                preview: Some(document.to_document()),
                areas,
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
        report,
    })
}

#[cfg(feature = "election_config_archive")]
#[derive(Serialize)]
struct PreviewOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    preview: Option<serde_json::Value>,
    areas: Vec<preview::PreviewArea>,
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

/// What a profile hides, so the wizard knows what not to draw.
///
/// Rust decides which *paths*, because that is a statement about the plan.
/// Which screens that empties is a question about screens, and belongs to
/// whoever draws them.
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
