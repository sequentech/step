// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::{anyhow, Context, Result};
use csv::Writer;
use roxmltree::{Document, Node};
use sequent_core::types::tally_sheet_import::TallySheetImportValidationError;
use sequent_core::types::tally_sheets::VotingChannel;
use tracing::instrument;

/// ES&S Enhanced XML files can carry vote totals for multiple reporting
/// groups (e.g. election-day vs. absentee); this is the reporting group id
/// read by default when a caller doesn't ask for a specific one.
pub const DEFAULT_IMPORT_REPORTING_GROUP_ID: &str = "1";

/// Annotation key recording which ES&S element the importer took each tally
/// sheet's area name from. Namespaced like the other integration-specific
/// annotation keys in this codebase (`datafix:*`, `miru:*`) so that a
/// vendor detail stays out of the product's own columns and types — this is
/// a derived record of what happened, never an input.
pub const ESS_AREA_GROUPING_ANNOTATION_KEY: &str = "ess:area_grouping";

/// Value recorded under `ESS_AREA_GROUPING_ANNOTATION_KEY`. Area names come
/// from `<Party name>` — e.g. a school-support group, for events whose Areas
/// are configured as ballot-style groups rather than physical polling
/// places. Recorded explicitly so the audit trail stays meaningful if
/// another grouping is ever supported again.
pub const AREA_GROUPING_PARTY: &str = "PARTY";

/// What a conversion produced: the canonical CSV, any validation errors, and
/// which ES&S element the area names were taken from (recorded on the import
/// for audit — see `ESS_AREA_GROUPING_ANNOTATION_KEY`).
#[derive(Debug, Clone)]
pub struct EssConversion {
    pub canonical_csv: Vec<u8>,
    pub validation_errors: Vec<TallySheetImportValidationError>,
    pub area_grouping: &'static str,
}

/// The STEP contest config this converter needs: `max_votes`, the "vote
/// for N" bound every derived figure depends on (see
/// `resolve_contest_max_votes`). A non-positive value is treated as `1`,
/// i.e. single-choice.
#[derive(Debug, Clone, Copy, Default)]
pub struct ContestVoteConfig {
    pub max_votes: i64,
}

/// What to do about a contest that the election event's contest config
/// doesn't mention at all. `max_votes` is not a detail — `implicit_invalid`,
/// `total_blank_votes` and `total_valid_votes` are all derived by dividing
/// ES&S's selection-slot counts by it — so guessing it wrong silently
/// corrupts every figure for that contest. Which way to resolve that
/// depends on whether an election event exists to be authoritative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingContestConfigPolicy {
    /// Offline conversion (`step-cli`): there is no election event, so the
    /// file's own `Contest@voteFor` is the only authority available.
    UseFileVoteFor,
    /// An election event exists: a contest it doesn't configure is a real
    /// disagreement between the file and the event, so the contest is
    /// skipped and reported rather than converted on a guessed `max_votes`.
    Reject,
}

/// A single party's totals for one contest, aggregated across every
/// precinct — see `resolve_contest_data_by_party`.
#[derive(Debug, Clone, Default)]
struct ContestPartyTotals {
    /// From the `OVERVOTES`/`UNDERVOTES` pseudo-candidates' party-
    /// attributed vote counts (raw selection-slot counts, not ballots) —
    /// see `resolve_contest_data_by_party`. Whole-ballot census/blanks for
    /// a party live separately in `party_ballots_and_blanks_by_id`'s
    /// result, not here — see `convert_party_grouped`.
    over_votes: u64,
    under_votes: u64,
}

/// A candidate's votes keyed by party id — see `candidate_votes_by_party`.
#[derive(Debug, Clone)]
struct CandidateVotes {
    external_id: String,
    votes_by_key: HashMap<String, u64>,
}

/// Builds a validation error for a problem scoped to a single `Contest`
/// element (or, if the contest couldn't even be identified yet, the whole
/// file). Mirrors the shape `parse_canonical_csv` uses for its own errors,
/// so XML and CSV import problems are reported identically to the caller.
fn xml_error(
    selected_channel: &VotingChannel,
    contest_external_id: Option<&str>,
    message: impl Into<String>,
) -> TallySheetImportValidationError {
    TallySheetImportValidationError {
        code: "xml_conversion_error".to_string(),
        message: message.into(),
        channel: Some(selected_channel.clone()),
        area_name: None,
        contest_external_id: contest_external_id.map(|id| id.to_string()),
        candidate_external_id: None,
        field: None,
        params: HashMap::new(),
    }
}

/// Builds a validation error for a party-grouping precondition that the
/// source file doesn't meet. Unlike `xml_error`, these are whole-file
/// problems — no contest can be converted safely once one holds — but they
/// are still the file's data failing a documented requirement, not a
/// malformed file, so they're reported through the same validation-error
/// channel as everything else (visible in the import preview, and recorded
/// on the import as `FAILED_VALIDATION`) rather than as an opaque request
/// failure. `params` carries the figures so a UI can render a translated
/// message; `message` is the pre-formatted English fallback.
fn party_precondition_error(
    selected_channel: &VotingChannel,
    code: &str,
    message: impl Into<String>,
    params: HashMap<String, String>,
) -> TallySheetImportValidationError {
    TallySheetImportValidationError {
        code: code.to_string(),
        message: message.into(),
        channel: Some(selected_channel.clone()),
        area_name: None,
        contest_external_id: None,
        candidate_external_id: None,
        field: None,
        params,
    }
}

/// Writes the canonical CSV header row. Shared so every conversion path —
/// including one refused before it reads a single contest — emits the same
/// well-formed CSV shape.
fn write_canonical_csv_header(writer: &mut Writer<Vec<u8>>) -> Result<()> {
    writer.write_record([
        "channel",
        "area_name",
        "contest_external_id",
        "field",
        "candidate_external_id",
        "value",
    ])?;
    Ok(())
}

/// Builds the validation error for a file whose party names match none of
/// the election event's Areas. Includes a few names from each side, since
/// the fix is almost always "these two lists were meant to be the same".
/// The file's precinct names are listed too: an event whose Areas are named
/// after precincts is the most likely reason for a total mismatch, and
/// seeing both lists makes that immediately obvious.
fn area_detection_error(
    precinct_names: &HashMap<String, String>,
    party_names: &HashMap<String, String>,
    configured_area_names: &HashSet<String>,
) -> TallySheetImportValidationError {
    /// Enough names to recognise the mismatch without flooding the UI.
    const SAMPLE_LIMIT: usize = 5;

    fn sample<'a>(names: impl Iterator<Item = &'a String>) -> String {
        let mut sampled: Vec<&str> = names.map(String::as_str).collect();
        sampled.sort_unstable();
        let total = sampled.len();
        sampled.truncate(SAMPLE_LIMIT);
        let listed = sampled.join(", ");
        if total > SAMPLE_LIMIT {
            format!("{listed}, … ({total} total)")
        } else {
            listed
        }
    }

    let precinct_sample = sample(precinct_names.values());
    let party_sample = sample(party_names.values());
    let configured_sample = sample(configured_area_names.iter());

    TallySheetImportValidationError {
        code: "ess_area_names_do_not_match_election_event".to_string(),
        message: format!(
            "None of this file's party names match the election event's areas, so no tally sheet could be assigned to an area. The event's areas are [{configured_sample}] and the file's parties are [{party_sample}]. For reference, the file's precincts are [{precinct_sample}] — areas named after precincts are not supported, they must be named after parties"
        ),
        channel: None,
        area_name: None,
        contest_external_id: None,
        candidate_external_id: None,
        field: None,
        params: HashMap::from([
            ("configuredAreaNames".to_string(), configured_sample),
            ("precinctNames".to_string(), precinct_sample),
            ("partyNames".to_string(), party_sample),
        ]),
    }
}

/// Checks the exact per-ballot accounting identity documented in the ES&S
/// EVS SOP for `overVotes`/`underVotes`: every one of a contest's
/// `max_votes` selection slots, across every ballot cast for it, ends up
/// as either a candidate mark, an overvote slot, or an undervote slot,
/// with no remainder — so `candidate_votes_sum + over_votes + under_votes`
/// must equal `total_votes * max_votes` exactly. Returns a validation
/// error scoped to this precinct/contest when it doesn't (a data-quality
/// problem in the source file, not something a caller can fix).
#[allow(clippy::too_many_arguments)]
fn check_vote_reconciliation(
    selected_channel: &VotingChannel,
    area_name: &str,
    contest_external_id: &str,
    candidate_votes_sum: u64,
    over_votes: u64,
    under_votes: u64,
    total_votes: u64,
    max_votes: u64,
) -> Option<TallySheetImportValidationError> {
    let expected = total_votes.saturating_mul(max_votes);
    let actual = candidate_votes_sum + over_votes + under_votes;
    if actual == expected {
        return None;
    }
    Some(TallySheetImportValidationError {
        code: "ess_vote_reconciliation_mismatch".to_string(),
        message: format!(
            "candidate votes ({candidate_votes_sum}) + over votes ({over_votes}) + under votes ({under_votes}) must equal total votes ({total_votes}) \u{d7} {max_votes} marks per ballot ({expected})"
        ),
        channel: Some(selected_channel.clone()),
        area_name: Some(area_name.to_string()),
        contest_external_id: Some(contest_external_id.to_string()),
        candidate_external_id: None,
        field: Some("total_valid_votes".to_string()),
        params: HashMap::from([
            ("candidateVotesSum".to_string(), candidate_votes_sum.to_string()),
            ("overVotes".to_string(), over_votes.to_string()),
            ("underVotes".to_string(), under_votes.to_string()),
            ("totalVotes".to_string(), total_votes.to_string()),
            ("maxVotes".to_string(), max_votes.to_string()),
            ("expected".to_string(), expected.to_string()),
        ]),
    })
}

/// Checks that `overVotes` is an exact multiple of `max_votes`, for the
/// `ContestReportingGroupVotes` variant. ES&S always attributes a whole
/// `max_votes` allotment to `overVotes` for every overvoted ballot
/// (confirmed by the EVS SOP and empirically, no partial-slot overvotes
/// observed) — so a non-exact remainder indicates a data-quality problem in
/// the source file, not something this importer can resolve on its own.
fn check_over_votes_divisible(
    selected_channel: &VotingChannel,
    area_name: &str,
    contest_external_id: &str,
    over_votes: u64,
    max_votes: u64,
) -> Option<TallySheetImportValidationError> {
    if over_votes.is_multiple_of(max_votes) {
        return None;
    }
    Some(TallySheetImportValidationError {
        code: "ess_over_votes_not_divisible".to_string(),
        message: format!(
            "over votes ({over_votes}) is not an exact multiple of {max_votes} marks per ballot — cannot determine the number of overvoted ballots"
        ),
        channel: Some(selected_channel.clone()),
        area_name: Some(area_name.to_string()),
        contest_external_id: Some(contest_external_id.to_string()),
        candidate_external_id: None,
        field: Some("implicit_invalid".to_string()),
        params: HashMap::from([
            ("overVotes".to_string(), over_votes.to_string()),
            ("maxVotes".to_string(), max_votes.to_string()),
        ]),
    })
}

/// Sanity-checks that a contest's derived `total_blank_votes` is at least
/// the party's whole-ballot blank count (`blanksCast`, read from
/// `PrecinctParty` — see `party_ballots_and_blanks_by_id`). Every ballot
/// that's blank on the whole ballot is necessarily blank in this contest
/// too, so `total_blank_votes` can never be smaller — a violation
/// indicates a data-quality problem in the source file, not something this
/// importer can resolve on its own.
fn check_blank_votes_at_least_precinct_minimum(
    selected_channel: &VotingChannel,
    area_name: &str,
    contest_external_id: &str,
    total_blank_votes: u64,
    precinct_blanks_cast: u64,
) -> Option<TallySheetImportValidationError> {
    if total_blank_votes >= precinct_blanks_cast {
        return None;
    }
    Some(TallySheetImportValidationError {
        code: "ess_blank_votes_below_precinct_minimum".to_string(),
        message: format!(
            "total blank votes ({total_blank_votes}) is less than the precinct's whole-ballot blank count ({precinct_blanks_cast}) — every ballot blank on the whole ballot must also be blank in this contest"
        ),
        channel: Some(selected_channel.clone()),
        area_name: Some(area_name.to_string()),
        contest_external_id: Some(contest_external_id.to_string()),
        candidate_external_id: None,
        field: Some("total_blank_votes".to_string()),
        params: HashMap::from([
            ("totalBlankVotes".to_string(), total_blank_votes.to_string()),
            ("precinctBlanksCast".to_string(), precinct_blanks_cast.to_string()),
        ]),
    })
}

/// Resolves the `max_votes` ("vote for N") bound to use for one contest,
/// reconciling the two authorities that can carry it: the election event's
/// contest config, and the file's own `Contest@voteFor`.
///
/// This matters more than its size suggests. `implicit_invalid`,
/// `total_blank_votes` and `total_valid_votes` are all
/// `over_votes`/`under_votes` divided by `max_votes`, so a wrong value
/// silently corrupts every derived figure for the contest — a "vote for 4"
/// contest read as single-choice reports 4x the invalid and blank ballots
/// it should, and `total_valid_votes` saturates to zero. Previously a
/// contest missing from the config defaulted to `1` with no signal at all,
/// and the damage only surfaced downstream as a confusing
/// `ess_vote_reconciliation_mismatch`.
///
/// Returns `Err(validation_error)` when the two authorities disagree, or
/// when there is no authority at all — the caller skips the contest rather
/// than emitting rows built on a guessed bound.
#[expect(
    clippy::result_large_err,
    reason = "Keep structured validation errors consistent with the importer aggregation API; boxing would change its error contract solely for a size heuristic."
)]
#[instrument(skip_all, err(Debug))]
fn resolve_contest_max_votes(
    contest: Node<'_, '_>,
    contest_external_id: &str,
    contest_vote_config: &HashMap<String, ContestVoteConfig>,
    missing_config_policy: MissingContestConfigPolicy,
    selected_channel: &VotingChannel,
) -> std::result::Result<u64, TallySheetImportValidationError> {
    // Absent or unparseable `voteFor` just means "no cross-check available"
    // — it is not required by every ES&S export, so its absence alone is
    // not a data problem.
    let file_vote_for: Option<u64> = contest
        .attribute("voteFor")
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|vote_for| *vote_for > 0);

    let configured_max_votes = contest_vote_config
        .get(contest_external_id)
        .map(|config| config.max_votes.max(1) as u64);

    match (configured_max_votes, file_vote_for) {
        (Some(configured), Some(vote_for)) if configured != vote_for => {
            Err(TallySheetImportValidationError {
                code: "ess_vote_for_does_not_match_contest_config".to_string(),
                message: format!(
                    "this file reports the contest as vote-for-{vote_for}, but the election event configures it as vote-for-{configured} — every derived figure (invalid, blank and valid vote counts) depends on that number, so the contest was skipped rather than converted with the wrong one"
                ),
                channel: Some(selected_channel.clone()),
                area_name: None,
                contest_external_id: Some(contest_external_id.to_string()),
                candidate_external_id: None,
                field: None,
                params: HashMap::from([
                    ("fileVoteFor".to_string(), vote_for.to_string()),
                    ("configuredMaxVotes".to_string(), configured.to_string()),
                ]),
            })
        }
        (Some(configured), _) => Ok(configured),
        (None, file_vote_for) => match missing_config_policy {
            MissingContestConfigPolicy::UseFileVoteFor => Ok(file_vote_for.unwrap_or(1)),
            MissingContestConfigPolicy::Reject => Err(TallySheetImportValidationError {
                code: "ess_contest_not_configured".to_string(),
                message: format!(
                    "contest '{contest_external_id}' is not configured in this election event, so its vote-for bound is unknown — every derived figure (invalid, blank and valid vote counts) depends on it, so the contest was skipped rather than converted with a guessed one"
                ),
                channel: Some(selected_channel.clone()),
                area_name: None,
                contest_external_id: Some(contest_external_id.to_string()),
                candidate_external_id: None,
                field: None,
                params: HashMap::from([(
                    "contestExternalId".to_string(),
                    contest_external_id.to_string(),
                )]),
            }),
        },
    }
}

/// Converts using the default reporting group.
///
/// This is the offline entry point (`step-cli`): with no election event
/// there are no configured Area names to check the file's party names
/// against, so it converts without that check. For the same reason a
/// contest missing from `contest_vote_config` falls back to the file's own
/// `Contest@voteFor` rather than being rejected — there is no election
/// event here to be authoritative about it. Anything running with an
/// election event calls
/// `convert_ess_enhanced_xml_to_csv_for_reporting_group`, which performs
/// both checks.
#[instrument(skip_all, err)]
pub fn convert_ess_enhanced_xml_to_csv(
    xml_bytes: &[u8],
    selected_channel: VotingChannel,
    contest_vote_config: &HashMap<String, ContestVoteConfig>,
) -> Result<(Vec<u8>, Vec<TallySheetImportValidationError>)> {
    let xml = std::str::from_utf8(xml_bytes).context("ES&S XML import must be valid UTF-8")?;
    let document = Document::parse(xml).context("Invalid ES&S Enhanced XML")?;
    convert_party_grouped(
        &document,
        selected_channel,
        DEFAULT_IMPORT_REPORTING_GROUP_ID,
        contest_vote_config,
        MissingContestConfigPolicy::UseFileVoteFor,
    )
}

/// Converts an ES&S Enhanced XML file to canonical tally sheet CSV.
///
/// Only genuinely file-wide problems (invalid UTF-8, unparseable XML, a
/// malformed `JurisdictionMap`) are hard failures. A structural problem
/// scoped to a single `Contest` (missing reporting-group data, a duplicate
/// or unresolved precinct reference, a non-numeric attribute, a missing
/// `altId1`) skips just that contest and is instead reported as a
/// `TallySheetImportValidationError`, exactly like `parse_canonical_csv`
/// does for a bad CSV row — so a file with one broken contest still
/// converts and imports every other contest in it.
///
/// `contest_vote_config` maps a contest's external id to its `max_votes`
/// (STEP contest config). `max_votes` is cross-checked against
/// the file's own `Contest@voteFor`, and a contest the event doesn't
/// configure at all is skipped rather than guessed at — see
/// `resolve_contest_max_votes` for why that number can't be defaulted
/// safely.
///
/// `total_votes` is each party's `<PrecinctParty ballotsCast>` — an
/// authoritative ballot count for every contest on that party's ballot,
/// since a party maps to exactly one ballot style — and `total_valid_votes`
/// is that figure minus the invalid ballots recovered from `overVotes`.
/// `census` uses the same party-wide ballots cast. See
/// `convert_party_grouped` for the full derivation.
///
/// Each tally sheet's `area_name` comes from `<Party name>`.
/// `configured_area_names` is the election event's Area names, checked up
/// front so a file naming nothing the event recognises reports once rather
/// than once per contest — see `check_party_names_match_configured_areas`.
#[instrument(skip_all, err)]
pub fn convert_ess_enhanced_xml_to_csv_for_reporting_group(
    xml_bytes: &[u8],
    selected_channel: VotingChannel,
    reporting_group_id: &str,
    contest_vote_config: &HashMap<String, ContestVoteConfig>,
    configured_area_names: &HashSet<String>,
) -> Result<EssConversion> {
    let xml = std::str::from_utf8(xml_bytes).context("ES&S XML import must be valid UTF-8")?;
    let document = Document::parse(xml).context("Invalid ES&S Enhanced XML")?;

    // Nothing in the file lines up with the event's Areas, so every row this
    // produced would fail to resolve. Report that once, up front, instead of
    // one "Area not found" per contest afterwards.
    if let Some(error) = check_party_names_match_configured_areas(&document, configured_area_names)?
    {
        return Ok(EssConversion {
            canonical_csv: empty_canonical_csv()?,
            validation_errors: vec![error],
            area_grouping: AREA_GROUPING_PARTY,
        });
    }

    let (canonical_csv, validation_errors) = convert_party_grouped(
        &document,
        selected_channel,
        reporting_group_id,
        contest_vote_config,
        MissingContestConfigPolicy::Reject,
    )?;

    Ok(EssConversion {
        canonical_csv,
        validation_errors,
        area_grouping: AREA_GROUPING_PARTY,
    })
}

/// Checks the file's `<Party name>` values against the election event's
/// configured Area names. Downstream resolution (`get_area_by_name`) is an
/// exact match, so this compares exactly too.
///
/// Matching *nothing* is not a hard failure but `Ok(Some(error))` — a
/// normal validation error, because it means the file and the event
/// disagree, which is exactly the kind of data problem the import UI exists
/// to show. Matching even one party is enough to proceed: a party with real
/// data whose name matches no Area still surfaces through `get_area_by_name`
/// individually, same as any other unrecognized area name.
#[instrument(skip_all, err)]
fn check_party_names_match_configured_areas(
    document: &Document<'_>,
    configured_area_names: &HashSet<String>,
) -> Result<Option<TallySheetImportValidationError>> {
    let party_names = party_names_by_id(document)?;

    if party_names
        .values()
        .any(|name| configured_area_names.contains(name))
    {
        return Ok(None);
    }

    let precinct_names = precinct_names_by_id(document)?;
    Ok(Some(area_detection_error(
        &precinct_names,
        &party_names,
        configured_area_names,
    )))
}

/// The canonical CSV header with no data rows, for a conversion refused
/// before any contest could be read.
fn empty_canonical_csv() -> Result<Vec<u8>> {
    let mut writer = Writer::from_writer(Vec::new());
    write_canonical_csv_header(&mut writer)?;
    writer.into_inner().map_err(|err| anyhow!(err))
}

/// Party-grouped ES&S conversion — `area_name` is each contest's party
/// name (e.g. a school-support group like "English Public"), read from
/// `<Party name>`, instead of the physical precinct. Every candidate
/// (including the `OVERVOTES`/`UNDERVOTES` pseudo-candidates) carries a
/// `<CandidatePrecinctSplitVotes refBStyleId>` breakdown alongside its
/// precinct-level figures; `refBStyleId` resolves to exactly one party via
/// `<PrecinctParty partyId><PrecinctPartySplit refBStyleId>` — see
/// `ballot_style_to_party_id`. Summing that breakdown gives an *exact*
/// per-party vote count, not an approximation, because ES&S emits it
/// (and each party's whole-ballot `<PrecinctParty ballotsCast/blanksCast>`
/// figures) redundantly for every candidate/precinct, unconditional on
/// which of the two "reporting group" XML variants the file otherwise uses
/// — verified against multi-party precincts in real Woodstock exports,
/// where a single precinct's per-candidate totals split cleanly across
/// several parties and sum back exactly.
///
/// Every contest's totals are therefore derived the same way regardless of
/// whether it's assigned to one area (e.g. a school-board trustee race,
/// structurally on only one party's ballot) or several (e.g. a municipal
/// race on every ballot). Party grouping has an authoritative ballot count:
/// a party maps to exactly one ballot style (see `ballot_style_to_party_id`),
/// so every contest on that ballot style is on every one of that party's
/// ballots — `<PrecinctParty ballotsCast>` is therefore this contest's
/// genuine ballot count too, not just a whole-ballot upper bound. So
/// `total_votes` is that authoritative figure, `over_votes`/`under_votes`
/// are selection-slot counts recovered into ballot counts via `/ max_votes`
/// (never derived from summing candidate marks, which can legitimately
/// exceed the ballot count on a "vote for N" contest), and
/// `check_vote_reconciliation`/`check_over_votes_divisible`/
/// `check_blank_votes_at_least_precinct_minimum` all run.
///
/// Neither `CandidatePrecinctSplitVotes` nor `PrecinctParty` carry a
/// `reportingGroupId` of their own, so this data can't be scoped to a
/// specific reporting group (e.g. election-day vs. absentee) —
/// `validate_party_data_matches_reporting_group` refuses the conversion up
/// front, unless each precinct's party data adds up to exactly the requested
/// group, rather than silently blending every channel's ballots into
/// whichever one was requested.
///
/// A `(contest, party)` pair only gets a row when it has some genuine
/// data (a candidate vote, an over-vote, or an under-vote); a party that
/// structurally never sees this contest on its ballot (e.g. every other
/// school-support group for a trustee race) always has all three at zero
/// — ES&S still emits a zero-valued split entry for it regardless of
/// relevance, so this avoids flooding the output with meaningless
/// zero-value rows. A party *with* real data whose name doesn't match any
/// configured Area still gets a row — Sequent's own existing area lookup
/// (`get_area_by_name`) surfaces that mismatch as a normal, visible
/// validation error at import time, same as it would for any other
/// unrecognized area name; this function doesn't need its own separate
/// check for it.
#[instrument(skip_all, err)]
fn convert_party_grouped(
    document: &Document<'_>,
    selected_channel: VotingChannel,
    reporting_group_id: &str,
    contest_vote_config: &HashMap<String, ContestVoteConfig>,
    missing_config_policy: MissingContestConfigPolicy,
) -> Result<(Vec<u8>, Vec<TallySheetImportValidationError>)> {
    let mut writer = Writer::from_writer(Vec::new());
    let mut validation_errors = Vec::new();

    write_canonical_csv_header(&mut writer)?;

    // Party grouping's preconditions are whole-file: once one fails, no
    // contest in the file can be converted safely. Return the empty
    // (header-only) CSV alongside the error rather than failing the request,
    // so the caller reports it exactly like every other validation error —
    // the preview shows it, and `create_tally_sheet_import` records the
    // import as FAILED_VALIDATION with the explanation attached.
    let precondition_error = validate_party_data_matches_reporting_group(
        document,
        reporting_group_id,
        &selected_channel,
    )?;
    if let Some(error) = precondition_error {
        return Ok((
            writer.into_inner().map_err(|err| anyhow!(err))?,
            vec![error],
        ));
    }
    let ballot_style_to_party = match ballot_style_to_party_id(document, &selected_channel)? {
        Ok(map) => map,
        Err(error) => {
            return Ok((
                writer.into_inner().map_err(|err| anyhow!(err))?,
                vec![error],
            ));
        }
    };

    let party_names = party_names_by_id(document)?;
    let party_ballots_and_blanks = party_ballots_and_blanks_by_id(document)?;

    for contest in document
        .descendants()
        .filter(|node| node.has_tag_name("Contest"))
    {
        let contest_external_id = match required_attr(contest, "altId1", "Contest") {
            Ok(id) if !id.trim().is_empty() => id,
            Ok(_) | Err(_) => {
                validation_errors.push(xml_error(
                    &selected_channel,
                    None,
                    "Contest is missing altId1 import id",
                ));
                continue;
            }
        };

        let (totals_by_party, candidates) =
            match resolve_contest_data_by_party(contest, &ballot_style_to_party) {
                Ok(data) => data,
                Err(error) => {
                    validation_errors.push(xml_error(
                        &selected_channel,
                        Some(&contest_external_id),
                        error.to_string(),
                    ));
                    continue;
                }
            };

        let max_votes = match resolve_contest_max_votes(
            contest,
            &contest_external_id,
            contest_vote_config,
            missing_config_policy,
            &selected_channel,
        ) {
            Ok(max_votes) => max_votes,
            Err(error) => {
                validation_errors.push(error);
                continue;
            }
        };

        for (party_id, totals) in totals_by_party {
            let Some(party_name) = party_names.get(&party_id) else {
                validation_errors.push(xml_error(
                    &selected_channel,
                    Some(&contest_external_id),
                    format!(
                        "Contest references party id '{}' not present in PartyMap",
                        party_id
                    ),
                ));
                continue;
            };

            let candidate_votes_sum = candidates
                .iter()
                .map(|candidate| candidate.votes_by_key.get(&party_id).copied().unwrap_or(0))
                .sum::<u64>();
            if candidate_votes_sum == 0 && totals.over_votes == 0 && totals.under_votes == 0 {
                // This party has no data at all for this contest — the
                // structurally-guaranteed case for a party that never sees
                // this contest on its ballot at all (ES&S still emits a
                // zero-valued split entry for it regardless of relevance).
                // Not an error, just not applicable here.
                continue;
            }

            let (party_ballots_cast, party_whole_ballot_blanks) = party_ballots_and_blanks
                .get(&party_id)
                .copied()
                .unwrap_or((0, 0));

            // `party_ballots_cast` is genuinely this contest's ballot count,
            // not just an upper bound, because a party maps to exactly one
            // ballot style (see ballot_style_to_party_id) — every contest on
            // that ballot style is on every one of that party's ballots,
            // with no partial appearance. So total_votes is that
            // authoritative figure, not derived from candidate marks —
            // candidate_votes_sum can legitimately exceed the ballot count
            // on a "vote for N" contest, so it must never be treated as a
            // ballot count (the bug this replaced: multi-select contests
            // were tripping the "total votes must not exceed census" check
            // downstream). over_votes/under_votes are selection-slot counts
            // here too (from the OVERVOTES/UNDERVOTES pseudo-candidates'
            // party-attributed marks), so they need the same /max_votes
            // recovery — see check_over_votes_divisible's doc comment.
            let total_votes = party_ballots_cast;
            let total_blank_votes = totals.under_votes / max_votes;
            let implicit_invalid = totals.over_votes / max_votes;
            let explicit_invalid = 0;
            let total_invalid = implicit_invalid + explicit_invalid;
            let total_valid_votes = total_votes.saturating_sub(total_invalid);

            if let Some(error) = check_over_votes_divisible(
                &selected_channel,
                party_name,
                &contest_external_id,
                totals.over_votes,
                max_votes,
            ) {
                validation_errors.push(error);
            }
            if let Some(error) = check_vote_reconciliation(
                &selected_channel,
                party_name,
                &contest_external_id,
                candidate_votes_sum,
                totals.over_votes,
                totals.under_votes,
                total_votes,
                max_votes,
            ) {
                validation_errors.push(error);
            }
            if let Some(error) = check_blank_votes_at_least_precinct_minimum(
                &selected_channel,
                party_name,
                &contest_external_id,
                total_blank_votes,
                party_whole_ballot_blanks,
            ) {
                validation_errors.push(error);
            }

            write_scalar_row(
                &mut writer,
                &selected_channel,
                party_name,
                &contest_external_id,
                "total_votes",
                total_votes,
            )?;
            write_scalar_row(
                &mut writer,
                &selected_channel,
                party_name,
                &contest_external_id,
                "total_valid_votes",
                total_valid_votes,
            )?;
            write_scalar_row(
                &mut writer,
                &selected_channel,
                party_name,
                &contest_external_id,
                "implicit_invalid",
                implicit_invalid,
            )?;
            write_scalar_row(
                &mut writer,
                &selected_channel,
                party_name,
                &contest_external_id,
                "explicit_invalid",
                explicit_invalid,
            )?;
            write_scalar_row(
                &mut writer,
                &selected_channel,
                party_name,
                &contest_external_id,
                "total_blank_votes",
                total_blank_votes,
            )?;
            // Whole-ballot blanks, unlike every other figure here, are a
            // property of the ballot box rather than of the contest: ES&S
            // reports `blanksCast` per party, and a party maps to exactly
            // one ballot style, so the same count is written onto every
            // contest sheet of the party — which is exactly how
            // `blank_ballots` is carried (see `validate_ballot_box_blank_ballots`).
            //
            // This is the one blank figure ES&S supplies directly. It needs
            // no `/ max_votes` recovery because it counts ballots, not
            // selection slots, and it is exact rather than the upper-bound
            // approximation `total_blank_votes` is for a "vote for N"
            // contest.
            write_scalar_row(
                &mut writer,
                &selected_channel,
                party_name,
                &contest_external_id,
                "blank_ballots",
                party_whole_ballot_blanks,
            )?;
            write_scalar_row(
                &mut writer,
                &selected_channel,
                party_name,
                &contest_external_id,
                "census",
                party_ballots_cast,
            )?;
            for candidate in &candidates {
                writer.write_record([
                    selected_channel.to_string(),
                    party_name.to_string(),
                    contest_external_id.clone(),
                    "candidate_votes".to_string(),
                    candidate.external_id.clone(),
                    candidate
                        .votes_by_key
                        .get(&party_id)
                        .copied()
                        .unwrap_or(0)
                        .to_string(),
                ])?;
            }
        }
    }

    let csv_bytes = writer.into_inner().map_err(|err| anyhow!(err))?;
    Ok((csv_bytes, validation_errors))
}

/// Validates that every precinct's party-level ballot count matches the
/// selected reporting group's, i.e. that the party data describes exactly
/// the channel being imported and nothing else.
///
/// Party-grouped conversion (`convert_party_grouped`) reads party-level
/// data (`PrecinctParty`, `PrecinctPartySplit`,
/// `CandidatePrecinctSplitVotes`) that ES&S doesn't scope to a reporting
/// group at all, unlike precinct-grouped conversion. So whenever any other
/// reporting group (e.g. an absentee/advance-voting channel) carries
/// ballots of its own, those ballots are silently present in the party
/// data too, and there is no way to subtract them back out — this refuses
/// the conversion outright rather than producing numbers that look
/// channel-specific but aren't.
///
/// Comparing `<PrecinctParty ballotsCast>` against the selected group
/// checks that dependency *directly*. (Comparing the "Total Votes" group
/// against the selected one, as an earlier version did, was only a proxy
/// for it — and silently became a no-op on files that omit the Total Votes
/// group entirely, which is exactly the shape produced by trimming a file
/// down to one reporting group.)
///
/// The comparison is an equality, not an upper bound: party data *smaller*
/// than the selected group means ballots attributed to no party at all,
/// which would under-count just as wrongly. A file with no `PrecinctParty`
/// data whatsoever therefore fails here too, with a single file-level
/// error rather than one confusing error per contest further down.
///
/// Returns `Ok(Some(error))` when the file doesn't meet the requirement —
/// a data problem, reported like any other validation error — and `Err`
/// only for a malformed element (a missing or non-numeric attribute).
#[instrument(skip_all, err)]
fn validate_party_data_matches_reporting_group(
    document: &Document<'_>,
    reporting_group_id: &str,
    selected_channel: &VotingChannel,
) -> Result<Option<TallySheetImportValidationError>> {
    for precinct in document
        .descendants()
        .filter(|node| node.has_tag_name("Precinct"))
    {
        let precinct_id = required_attr(precinct, "id", "Precinct")?;

        let mut selected_group_ballots_cast = 0;
        for group in precinct
            .children()
            .filter(|node| node.has_tag_name("PrecinctReportingGroup"))
        {
            let group_id = required_attr(group, "reportingGroupId", "PrecinctReportingGroup")?;
            if group_id == reporting_group_id {
                selected_group_ballots_cast =
                    parse_u64_attr(group, "ballotsCast", "PrecinctReportingGroup")?;
            }
        }

        let mut party_ballots_cast = 0;
        for precinct_party in precinct
            .children()
            .filter(|node| node.has_tag_name("PrecinctParty"))
        {
            party_ballots_cast += parse_u64_attr(precinct_party, "ballotsCast", "PrecinctParty")?;
        }

        if party_ballots_cast != selected_group_ballots_cast {
            return Ok(Some(party_precondition_error(
                selected_channel,
                "ess_party_data_not_scoped_to_reporting_group",
                format!(
                    "Party-grouped ES&S import requires each precinct's party data to describe exactly reporting group '{}', but precinct id '{}' has {} ballots across its PrecinctParty entries versus {} in reporting group '{}' — ES&S does not scope party-level data to a reporting group, so another reporting group's ballots (e.g. an absentee/advance-voting channel) cannot be separated out",
                    reporting_group_id,
                    precinct_id,
                    party_ballots_cast,
                    selected_group_ballots_cast,
                    reporting_group_id
                ),
                HashMap::from([
                    ("reportingGroupId".to_string(), reporting_group_id.to_string()),
                    ("precinctId".to_string(), precinct_id),
                    ("partyBallotsCast".to_string(), party_ballots_cast.to_string()),
                    (
                        "reportingGroupBallotsCast".to_string(),
                        selected_group_ballots_cast.to_string(),
                    ),
                ]),
            )));
        }
    }
    Ok(None)
}

/// Party id -> party name, from `<PartyMap><Party id name>`.
#[instrument(skip_all, err)]
fn party_names_by_id(document: &Document<'_>) -> Result<HashMap<String, String>> {
    let mut party_names = HashMap::new();
    for party in document
        .descendants()
        .filter(|node| node.has_tag_name("Party"))
    {
        let id = required_attr(party, "id", "Party")?;
        let name = required_attr(party, "name", "Party")?;
        party_names.insert(id, name);
    }
    Ok(party_names)
}

/// Ballot style id -> party id, from every `<PrecinctParty
/// partyId>`'s nested `<PrecinctPartySplit refBStyleId>`, across every
/// precinct. Enforces that party and ballot style are one-to-one across
/// the whole file, in *both* directions:
///
/// - A ballot style resolving to two different parties is straightforwardly
///   inconsistent source data, not something this importer can silently
///   pick a side on.
/// - A party owning two different ballot styles breaks the assumption
///   `convert_party_grouped` relies on to use `<PrecinctParty ballotsCast>`
///   as a contest's ballot count: that every contest on a party's ballot is
///   on *all* of that party's ballots. With two styles, a contest appearing
///   on only one of them (e.g. a ward-specific race) would silently inflate
///   `total_votes`/`total_valid_votes`. This is a stronger condition than
///   strictly necessary — two styles carrying identical contests would be
///   fine — but it is cheap to check and fails loudly instead of producing
///   quietly wrong totals; it can be relaxed if such a file ever appears.
///
/// Either violation is a data problem rather than a malformed file, so it
/// comes back as `Ok(Err(validation_error))` and is surfaced in the import
/// UI. `Err` is reserved for a malformed element (a missing attribute).
#[allow(clippy::type_complexity)]
#[instrument(skip_all, err)]
fn ballot_style_to_party_id(
    document: &Document<'_>,
    selected_channel: &VotingChannel,
) -> Result<std::result::Result<HashMap<String, String>, TallySheetImportValidationError>> {
    let mut party_by_style: HashMap<String, String> = HashMap::new();
    let mut style_by_party: HashMap<String, String> = HashMap::new();
    for precinct_party in document
        .descendants()
        .filter(|node| node.has_tag_name("PrecinctParty"))
    {
        let party_id = required_attr(precinct_party, "partyId", "PrecinctParty")?;
        for split in precinct_party
            .children()
            .filter(|node| node.has_tag_name("PrecinctPartySplit"))
        {
            let style_id = required_attr(split, "refBStyleId", "PrecinctPartySplit")?;
            match party_by_style.get(&style_id) {
                Some(existing) if existing != &party_id => {
                    return Ok(Err(party_precondition_error(
                        selected_channel,
                        "ess_ballot_style_maps_to_multiple_parties",
                        format!(
                            "Ballot style id '{}' maps to both party id '{}' and party id '{}' — inconsistent PrecinctPartySplit data",
                            style_id, existing, party_id
                        ),
                        HashMap::from([
                            ("ballotStyleId".to_string(), style_id.clone()),
                            ("firstPartyId".to_string(), existing.clone()),
                            ("secondPartyId".to_string(), party_id.clone()),
                        ]),
                    )));
                }
                _ => {
                    party_by_style.insert(style_id.clone(), party_id.clone());
                }
            }
            match style_by_party.get(&party_id) {
                Some(existing) if existing != &style_id => {
                    return Ok(Err(party_precondition_error(
                        selected_channel,
                        "ess_party_maps_to_multiple_ballot_styles",
                        format!(
                            "Party id '{}' maps to both ballot style id '{}' and ballot style id '{}' — party-grouped import requires exactly one ballot style per party, since it reads each party's whole-ballot count as the ballot count of every contest on that party's ballot",
                            party_id, existing, style_id
                        ),
                        HashMap::from([
                            ("partyId".to_string(), party_id.clone()),
                            ("firstBallotStyleId".to_string(), existing.clone()),
                            ("secondBallotStyleId".to_string(), style_id.clone()),
                        ]),
                    )));
                }
                _ => {
                    style_by_party.insert(party_id.clone(), style_id);
                }
            }
        }
    }
    Ok(Ok(party_by_style))
}

/// Sums each party's whole-ballot `ballotsCast`/`blanksCast` (as
/// `(ballots_cast, blanks_cast)`) across every precinct, from `<PrecinctParty
/// partyId ballotsCast blanksCast>`. Like `CandidatePrecinctSplitVotes`,
/// this isn't scoped to a reporting group — which is exactly what
/// `validate_party_data_matches_reporting_group` checks before any of it is
/// trusted.
#[instrument(skip_all, err)]
fn party_ballots_and_blanks_by_id(document: &Document<'_>) -> Result<HashMap<String, (u64, u64)>> {
    let mut totals: HashMap<String, (u64, u64)> = HashMap::new();
    for precinct_party in document
        .descendants()
        .filter(|node| node.has_tag_name("PrecinctParty"))
    {
        let party_id = required_attr(precinct_party, "partyId", "PrecinctParty")?;
        let ballots_cast = parse_u64_attr(precinct_party, "ballotsCast", "PrecinctParty")?;
        let blanks_cast = parse_u64_attr(precinct_party, "blanksCast", "PrecinctParty")?;
        let entry = totals.entry(party_id).or_default();
        entry.0 += ballots_cast;
        entry.1 += blanks_cast;
    }
    Ok(totals)
}

/// Resolves a single contest's per-party totals and candidate votes for
/// party-grouped conversion — see `convert_party_grouped`'s doc comment.
#[instrument(skip_all, err)]
fn resolve_contest_data_by_party(
    contest: Node<'_, '_>,
    ballot_style_to_party_id: &HashMap<String, String>,
) -> Result<(BTreeMap<String, ContestPartyTotals>, Vec<CandidateVotes>)> {
    let mut totals_by_party: BTreeMap<String, ContestPartyTotals> = BTreeMap::new();
    let mut candidates = Vec::new();

    for candidate in contest
        .children()
        .filter(|node| node.has_tag_name("Candidate"))
    {
        let candidate_type = candidate.attribute("type").unwrap_or("NORMAL");
        let votes_by_party = candidate_votes_by_party(candidate, ballot_style_to_party_id)?;

        // Every party that appears in this candidate's split gets a totals
        // entry, even at 0 votes — ES&S emits a zero-valued split entry
        // for every party regardless of relevance, so this reliably seeds
        // every party this contest could plausibly apply to.
        for party_id in votes_by_party.keys() {
            totals_by_party.entry(party_id.clone()).or_default();
        }

        if candidate_type == "OVERVOTES" {
            for (party_id, votes) in votes_by_party {
                totals_by_party.entry(party_id).or_default().over_votes += votes;
            }
            continue;
        }
        if candidate_type == "UNDERVOTES" {
            for (party_id, votes) in votes_by_party {
                totals_by_party.entry(party_id).or_default().under_votes += votes;
            }
            continue;
        }

        let external_id = required_attr(candidate, "altId1", "Candidate")?;
        if external_id.trim().is_empty() {
            return Err(anyhow!("Candidate is missing altId1 import id"));
        }
        candidates.push(CandidateVotes {
            external_id,
            votes_by_key: votes_by_party,
        });
    }

    Ok((totals_by_party, candidates))
}

/// Reads a candidate's `<CandidatePrecinctVotes><CandidatePrecinctSplitVotes
/// refBStyleId votes>` breakdown across every precinct, resolves each
/// ballot style to its party, and sums into a per-party vote count.
#[instrument(skip_all, err)]
fn candidate_votes_by_party(
    candidate: Node<'_, '_>,
    ballot_style_to_party_id: &HashMap<String, String>,
) -> Result<HashMap<String, u64>> {
    let mut votes_by_party: HashMap<String, u64> = HashMap::new();
    let mut seen_precincts = HashSet::new();
    for precinct_votes in candidate
        .children()
        .filter(|node| node.has_tag_name("CandidatePrecinctVotes"))
    {
        let precinct_id = required_attr(precinct_votes, "refPrecinctId", "CandidatePrecinctVotes")?;
        if !seen_precincts.insert(precinct_id.clone()) {
            return Err(anyhow!(
                "Duplicate CandidatePrecinctVotes for precinct id '{}'",
                precinct_id
            ));
        }
        for split in precinct_votes
            .children()
            .filter(|node| node.has_tag_name("CandidatePrecinctSplitVotes"))
        {
            let style_id = required_attr(split, "refBStyleId", "CandidatePrecinctSplitVotes")?;
            let vote_count = parse_u64_attr(split, "votes", "CandidatePrecinctSplitVotes")?;
            let Some(party_id) = ballot_style_to_party_id.get(&style_id) else {
                return Err(anyhow!(
                    "CandidatePrecinctSplitVotes references ballot style id '{}' not present in any PrecinctPartySplit",
                    style_id
                ));
            };
            *votes_by_party.entry(party_id.clone()).or_default() += vote_count;
        }
    }
    if votes_by_party.is_empty() {
        return Err(anyhow!(
            "Candidate is missing CandidatePrecinctSplitVotes party-level data required for party-grouped conversion"
        ));
    }
    Ok(votes_by_party)
}

#[instrument(skip_all, err)]
fn precinct_names_by_id(document: &Document<'_>) -> Result<HashMap<String, String>> {
    let mut precinct_names = HashMap::new();
    for precinct in document
        .descendants()
        .filter(|node| node.has_tag_name("Precinct"))
    {
        let id = required_attr(precinct, "id", "Precinct")?;
        let name = required_attr(precinct, "name", "Precinct")?;
        precinct_names.insert(id, name);
    }
    Ok(precinct_names)
}

fn write_scalar_row(
    writer: &mut Writer<Vec<u8>>,
    channel: &VotingChannel,
    area_name: &str,
    contest_external_id: &str,
    field: &str,
    value: u64,
) -> Result<()> {
    writer.write_record([
        channel.to_string(),
        area_name.to_string(),
        contest_external_id.to_string(),
        field.to_string(),
        String::new(),
        value.to_string(),
    ])?;
    Ok(())
}

fn required_attr(node: Node<'_, '_>, attribute: &str, context: &str) -> Result<String> {
    node.attribute(attribute)
        .map(|value| value.to_string())
        .ok_or_else(|| anyhow!("{context} is missing required attribute '{attribute}'"))
}

fn parse_u64_attr(node: Node<'_, '_>, attribute: &str, context: &str) -> Result<u64> {
    required_attr(node, attribute, context)?
        .parse::<u64>()
        .with_context(|| format!("{context} attribute '{attribute}' must be an unsigned integer"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A conversion refused by a whole-file party-grouping precondition
    /// still returns well-formed CSV — just the header and no data rows.
    fn csv_has_no_rows(csv: &[u8]) -> bool {
        String::from_utf8(csv.to_vec()).unwrap().lines().count() == 1
    }

    /// Area names matching the `<Party name>` values the fixtures below use,
    /// so the file's parties resolve to configured Areas.
    fn party_area_names() -> HashSet<String> {
        ["Area A", "Area B"].into_iter().map(String::from).collect()
    }

    /// Declares the named contests as single-choice ("vote for 1"). Every
    /// contest an election-event conversion touches has to be configured
    /// now — a contest missing from the map is rejected rather than
    /// silently defaulted to 1, since guessing `max_votes` corrupts every
    /// derived figure (see `resolve_contest_max_votes`).
    fn single_choice_config(contest_external_ids: &[&str]) -> HashMap<String, ContestVoteConfig> {
        contest_external_ids
            .iter()
            .map(|contest_external_id| {
                (
                    contest_external_id.to_string(),
                    ContestVoteConfig { max_votes: 1 },
                )
            })
            .collect()
    }

    /// Most tests only care about the CSV and the validation errors, not the
    /// recorded area grouping (that is asserted on its own below).
    fn convert_for_test(
        xml: &[u8],
        selected_channel: VotingChannel,
        contest_vote_config: &HashMap<String, ContestVoteConfig>,
        configured_area_names: &HashSet<String>,
    ) -> Result<(Vec<u8>, Vec<TallySheetImportValidationError>)> {
        let conversion = convert_ess_enhanced_xml_to_csv_for_reporting_group(
            xml,
            selected_channel,
            DEFAULT_IMPORT_REPORTING_GROUP_ID,
            contest_vote_config,
            configured_area_names,
        )?;
        Ok((conversion.canonical_csv, conversion.validation_errors))
    }

    /// A minimal well-formed file carrying both precinct and party data, so
    /// the area-name check below can be exercised against either list.
    const PARTY_AND_PRECINCT_FILE: &[u8] = br#"
        <ElectionReport>
            <JurisdictionMap>
                <Precinct id="p1" name="Precinct 1">
                    <PrecinctReportingGroup reportingGroupId="1" ballotsCast="4" blanksCast="0" />
                    <PrecinctParty partyId="1" ballotsCast="4" blanksCast="0">
                        <PrecinctPartySplit refBStyleId="b1" ballotsCast="4" blanksCast="0" />
                    </PrecinctParty>
                </Precinct>
            </JurisdictionMap>
            <PartyMap>
                <Party id="1" name="Area A" />
            </PartyMap>
            <Contest altId1="contest-1">
                <ContestReportingGroup reportingGroupId="1">
                    <ContestReportingGroupVotes refPrecinctId="p1" ballotsCast="4" overVotes="0" underVotes="0" blankVotes="0" />
                </ContestReportingGroup>
                <Candidate altId1="cand-1" type="NORMAL">
                    <CandidatePrecinctVotes refPrecinctId="p1" votes="4">
                        <CandidatePrecinctSplitVotes refBStyleId="b1" votes="4" />
                    </CandidatePrecinctVotes>
                </Candidate>
            </Contest>
        </ElectionReport>
    "#;

    fn convert_with_area_names(area_names: &HashSet<String>) -> EssConversion {
        convert_ess_enhanced_xml_to_csv_for_reporting_group(
            PARTY_AND_PRECINCT_FILE,
            VotingChannel::PAPER,
            DEFAULT_IMPORT_REPORTING_GROUP_ID,
            &single_choice_config(&["contest-1"]),
            area_names,
        )
        .unwrap()
    }

    #[test]
    fn uses_party_names_as_area_names() {
        let conversion = convert_with_area_names(&party_area_names());

        assert_eq!(conversion.area_grouping, "PARTY");
        assert!(conversion.validation_errors.is_empty());
        let csv = String::from_utf8(conversion.canonical_csv).unwrap();
        assert!(csv.contains("PAPER,Area A,contest-1,candidate_votes,cand-1,4"));
    }

    #[test]
    fn reports_one_error_when_no_area_name_matches_the_election_event() {
        // The file and the event disagree entirely. That has to be said once,
        // clearly, rather than as one "Area not found" per contest later.
        let unrelated: HashSet<String> = ["Somewhere Else".to_string()].into_iter().collect();
        let conversion = convert_with_area_names(&unrelated);

        assert_eq!(conversion.validation_errors.len(), 1);
        let error = &conversion.validation_errors[0];
        assert_eq!(error.code, "ess_area_names_do_not_match_election_event");
        assert!(error.message.contains("Area A"));
        assert!(error.message.contains("Somewhere Else"));
        // The file's precincts are listed too, since Areas named after
        // precincts are the most likely reason for a total mismatch.
        assert!(error.message.contains("Precinct 1"));
        assert!(csv_has_no_rows(&conversion.canonical_csv));
    }

    #[test]
    fn matching_a_single_party_is_enough_to_convert() {
        // Only one of the file's parties is a configured Area. That is not a
        // file-wide mismatch — the other party's rows surface individually
        // through get_area_by_name downstream, like any unrecognized name.
        let one_match: HashSet<String> = ["Area A".to_string()].into_iter().collect();
        let conversion = convert_with_area_names(&one_match);

        assert!(conversion.validation_errors.is_empty());
        assert!(!csv_has_no_rows(&conversion.canonical_csv));
    }

    #[test]
    fn offline_conversion_without_an_election_event_is_party_grouped() {
        // step-cli has no election event, so there are no Area names to check
        // the file's parties against — it converts without that check, and
        // falls back to the file's own voteFor for max_votes.
        let (csv, errors) = convert_ess_enhanced_xml_to_csv(
            PARTY_AND_PRECINCT_FILE,
            VotingChannel::PAPER,
            &HashMap::new(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        assert!(csv.contains("PAPER,Area A,contest-1,candidate_votes,cand-1,4"));
    }

    /// A "vote for 4" contest shaped like the CC race in a real Woodstock
    /// export: 3 ballots, 4 candidate marks, and overVotes/underVotes of 4
    /// each (selection-slot counts, i.e. one overvoted and one blank
    /// ballot). Reconciles exactly at max_votes 4: 4 + 4 + 4 == 3 * 4.
    const VOTE_FOR_FOUR_FILE: &[u8] = br#"
        <ElectionReport>
            <JurisdictionMap>
                <Precinct id="p1" name="Precinct 1">
                    <PrecinctReportingGroup reportingGroupId="1" ballotsCast="3" blanksCast="1" />
                    <PrecinctParty partyId="1" ballotsCast="3" blanksCast="1">
                        <PrecinctPartySplit refBStyleId="b1" ballotsCast="3" blanksCast="1" />
                    </PrecinctParty>
                </Precinct>
            </JurisdictionMap>
            <PartyMap>
                <Party id="1" name="Area A" />
            </PartyMap>
            <Contest altId1="CC" voteFor="4">
                <Candidate altId1="CCC1" type="NORMAL">
                    <CandidatePrecinctVotes refPrecinctId="p1" votes="1">
                        <CandidatePrecinctSplitVotes refBStyleId="b1" votes="1" />
                    </CandidatePrecinctVotes>
                </Candidate>
                <Candidate altId1="CCC2" type="NORMAL">
                    <CandidatePrecinctVotes refPrecinctId="p1" votes="1">
                        <CandidatePrecinctSplitVotes refBStyleId="b1" votes="1" />
                    </CandidatePrecinctVotes>
                </Candidate>
                <Candidate altId1="CCC3" type="NORMAL">
                    <CandidatePrecinctVotes refPrecinctId="p1" votes="1">
                        <CandidatePrecinctSplitVotes refBStyleId="b1" votes="1" />
                    </CandidatePrecinctVotes>
                </Candidate>
                <Candidate altId1="CCC4" type="NORMAL">
                    <CandidatePrecinctVotes refPrecinctId="p1" votes="1">
                        <CandidatePrecinctSplitVotes refBStyleId="b1" votes="1" />
                    </CandidatePrecinctVotes>
                </Candidate>
                <Candidate altId1="ovr" type="OVERVOTES">
                    <CandidatePrecinctVotes refPrecinctId="p1" votes="4">
                        <CandidatePrecinctSplitVotes refBStyleId="b1" votes="4" />
                    </CandidatePrecinctVotes>
                </Candidate>
                <Candidate altId1="und" type="UNDERVOTES">
                    <CandidatePrecinctVotes refPrecinctId="p1" votes="4">
                        <CandidatePrecinctSplitVotes refBStyleId="b1" votes="4" />
                    </CandidatePrecinctVotes>
                </Candidate>
            </Contest>
        </ElectionReport>
    "#;

    #[test]
    fn skips_contest_whose_vote_for_disagrees_with_the_configured_max_votes() {
        // The file says vote-for-4, the event says vote-for-1. Converting on
        // the wrong bound is what silently corrupts every derived figure
        // (implicit_invalid 4 instead of 1, total_valid_votes saturating to
        // 0), so the contest is skipped and the disagreement named directly
        // rather than surfacing later as a confusing reconciliation error.
        let (csv, errors) = convert_for_test(
            VOTE_FOR_FOUR_FILE,
            VotingChannel::PAPER,
            &single_choice_config(&["CC"]),
            &party_area_names(),
        )
        .unwrap();

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "ess_vote_for_does_not_match_contest_config");
        assert_eq!(errors[0].contest_external_id, Some("CC".to_string()));
        assert_eq!(errors[0].params.get("fileVoteFor"), Some(&"4".to_string()));
        assert_eq!(
            errors[0].params.get("configuredMaxVotes"),
            Some(&"1".to_string())
        );
        assert!(csv_has_no_rows(&csv));
    }

    #[test]
    fn skips_contest_absent_from_the_election_events_contest_config() {
        // Nothing configures this contest, so its vote-for bound is unknown.
        // Defaulting it to 1 is what previously turned a config mismatch
        // into silently wrong numbers.
        let (csv, errors) = convert_for_test(
            VOTE_FOR_FOUR_FILE,
            VotingChannel::PAPER,
            &single_choice_config(&["some-other-contest"]),
            &party_area_names(),
        )
        .unwrap();

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "ess_contest_not_configured");
        assert_eq!(errors[0].contest_external_id, Some("CC".to_string()));
        assert!(csv_has_no_rows(&csv));
    }

    #[test]
    fn offline_conversion_falls_back_to_the_files_vote_for() {
        // step-cli has no election event to configure contests, so the
        // file's own voteFor is the only authority — it must be used rather
        // than rejecting every contest or defaulting to 1.
        let (csv, errors) = convert_ess_enhanced_xml_to_csv(
            VOTE_FOR_FOUR_FILE,
            VotingChannel::PAPER,
            &HashMap::new(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        // Derived at max_votes 4, not 1.
        assert!(csv.contains("PAPER,Area A,CC,implicit_invalid,,1"));
        assert!(csv.contains("PAPER,Area A,CC,total_blank_votes,,1"));
        assert!(csv.contains("PAPER,Area A,CC,total_valid_votes,,2"));
    }

    #[test]
    fn reports_a_validation_error_when_totals_do_not_reconcile() {
        // candidate_sum(5) + over(1) + under(1) = 7, but party ballotsCast(9)
        // * max_votes(1, the default) = 9 — a genuine data-quality problem in
        // the source file, per the ES&S SOP's per-ballot accounting identity.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="9" blanksCast="0" />
                        <PrecinctParty partyId="1" ballotsCast="9" blanksCast="0">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="9" blanksCast="0" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                </PartyMap>
                <Contest altId1="contest-1">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="5">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="5" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="ovr" type="OVERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="1">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="1" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="und" type="UNDERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="1">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="1" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &single_choice_config(&["contest-1"]),
            &party_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "ess_vote_reconciliation_mismatch");
        assert_eq!(errors[0].contest_external_id, Some("contest-1".to_string()));
        assert_eq!(errors[0].area_name, Some("Area A".to_string()));
        // The rest of the contest still converts — this is a validation
        // warning about the source data, not a hard failure.
        assert!(csv.contains("PAPER,Area A,contest-1,candidate_votes,cand-1,5"));
    }

    #[test]
    fn reports_a_validation_error_when_over_votes_is_not_an_exact_multiple_of_max_votes() {
        // candidate_sum(3) + over(5) + under(4) = 12 = ballotsCast(3) *
        // max_votes(4), so the reconciliation identity holds — but overVotes
        // (5) isn't itself an exact multiple of max_votes (4), so there's no
        // way to know how many ballots it represents. This is a distinct
        // data-quality problem from a reconciliation mismatch.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="3" blanksCast="0" />
                        <PrecinctParty partyId="1" ballotsCast="3" blanksCast="0">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="3" blanksCast="0" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                </PartyMap>
                <Contest altId1="contest-1">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="3">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="3" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="ovr" type="OVERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="5">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="5" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="und" type="UNDERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="4">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="4" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let contest_vote_config =
            HashMap::from([("contest-1".to_string(), ContestVoteConfig { max_votes: 4 })]);
        let (_csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &contest_vote_config,
            &party_area_names(),
        )
        .unwrap();

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "ess_over_votes_not_divisible");
        assert_eq!(errors[0].contest_external_id, Some("contest-1".to_string()));
        assert_eq!(errors[0].area_name, Some("Area A".to_string()));
    }

    #[test]
    fn reports_a_validation_error_when_blank_votes_undercount_the_partys_whole_ballot_blanks() {
        // The party's whole-ballot blanksCast (2) means at least 2 ballots are
        // blank in every contest on that ballot, including this one — but this
        // contest's own underVotes (2) divided by max_votes (2) only accounts
        // for 1 blank ballot, an internally inconsistent combination that
        // can't reflect genuine source data.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="10" blanksCast="2" />
                        <PrecinctParty partyId="1" ballotsCast="10" blanksCast="2">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="10" blanksCast="2" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                </PartyMap>
                <Contest altId1="contest-1">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="18">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="18" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="und" type="UNDERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="2">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="2" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let contest_vote_config =
            HashMap::from([("contest-1".to_string(), ContestVoteConfig { max_votes: 2 })]);
        let (_csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &contest_vote_config,
            &party_area_names(),
        )
        .unwrap();

        // candidate_sum(18) + over(0) + under(2) == ballotsCast(10) * max_votes(2),
        // so the reconciliation and divisibility checks pass — only the
        // blanksCast lower-bound check fires.
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "ess_blank_votes_below_precinct_minimum");
        assert_eq!(errors[0].contest_external_id, Some("contest-1".to_string()));
        assert_eq!(errors[0].area_name, Some("Area A".to_string()));
    }

    #[test]
    fn carries_the_partys_whole_ballot_blank_count_onto_every_contest_sheet() {
        // `blanksCast` is the one blank figure ES&S supplies as a ballot
        // count rather than a selection-slot count: 2 of this party's 10
        // ballots were left blank on the whole ballot. Because a party maps
        // to exactly one ballot style, that count applies to the ballot box
        // as a whole and is replicated onto every contest sheet of the
        // party, which is how `blank_ballots` is carried.
        //
        // It is deliberately distinct from each contest's own
        // `total_blank_votes` (4 and 3 here): a ballot blank in one contest
        // may still carry a selection in another, so the per-contest counts
        // only bound the whole-ballot count from above -- they cannot
        // determine it. Here they leave the range 0..=3, so nothing but the
        // file itself could supply the answer of 2.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="0" ballotsCast="10" blanksCast="2" />
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="10" blanksCast="2" />
                        <PrecinctParty partyId="1" ballotsCast="10" blanksCast="2">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="10" blanksCast="2" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                </PartyMap>
                <Contest altId1="contest-1">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="5">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="5" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="ovr" type="OVERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="1">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="1" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="und" type="UNDERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="4">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="4" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
                <Contest altId1="contest-2">
                    <Candidate altId1="cand-2" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="7">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="7" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="und" type="UNDERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="3">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="3" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &single_choice_config(&["contest-1", "contest-2"]),
            &party_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        // The same whole-ballot count on both contest sheets of the box.
        assert!(csv.contains("PAPER,Area A,contest-1,blank_ballots,,2"));
        assert!(csv.contains("PAPER,Area A,contest-2,blank_ballots,,2"));
        // Each contest keeps its own, larger, per-contest blank count.
        assert!(csv.contains("PAPER,Area A,contest-1,total_blank_votes,,4"));
        assert!(csv.contains("PAPER,Area A,contest-2,total_blank_votes,,3"));
    }

    #[test]
    fn party_grouping_produces_one_row_for_single_area_contest() {
        // party 2 ("Area B") never sees this contest on its ballot at all
        // — every split entry for it is 0 — so only "Area A" gets a row.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="0" ballotsCast="10" blanksCast="0" />
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="10" blanksCast="0" />
                        <PrecinctParty partyId="1" ballotsCast="10" blanksCast="1">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="10" blanksCast="1" />
                        </PrecinctParty>
                        <PrecinctParty partyId="2" ballotsCast="0" blanksCast="0">
                            <PrecinctPartySplit refBStyleId="b2" ballotsCast="0" blanksCast="0" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                    <Party id="2" name="Area B" />
                </PartyMap>
                <Contest altId1="contest-1">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="8">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="8" />
                            <CandidatePrecinctSplitVotes refBStyleId="b2" votes="0" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="ovr" type="OVERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="1">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="1" />
                            <CandidatePrecinctSplitVotes refBStyleId="b2" votes="0" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="und" type="UNDERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="1">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="1" />
                            <CandidatePrecinctSplitVotes refBStyleId="b2" votes="0" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &single_choice_config(&["contest-1"]),
            &party_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        assert!(csv.contains("PAPER,Area A,contest-1,total_votes,,10"));
        assert!(csv.contains("PAPER,Area A,contest-1,total_valid_votes,,9"));
        assert!(csv.contains("PAPER,Area A,contest-1,implicit_invalid,,1"));
        assert!(csv.contains("PAPER,Area A,contest-1,total_blank_votes,,1"));
        assert!(csv.contains("PAPER,Area A,contest-1,census,,10"));
        assert!(csv.contains("PAPER,Area A,contest-1,candidate_votes,cand-1,8"));
        assert!(!csv.contains("Area B"));
    }

    #[test]
    fn party_grouping_splits_multi_area_contest_across_parties() {
        // A single precinct with ballots from two different parties for
        // the same contest (e.g. a municipal-wide race) — verifies the
        // per-candidate refBStyleId split correctly disaggregates by party
        // rather than mixing them.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="0" ballotsCast="10" blanksCast="0" />
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="10" blanksCast="0" />
                        <PrecinctParty partyId="1" ballotsCast="6" blanksCast="1">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="6" blanksCast="1" />
                        </PrecinctParty>
                        <PrecinctParty partyId="2" ballotsCast="4" blanksCast="0">
                            <PrecinctPartySplit refBStyleId="b2" ballotsCast="4" blanksCast="0" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                    <Party id="2" name="Area B" />
                </PartyMap>
                <Contest altId1="mayor">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="7">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="4" />
                            <CandidatePrecinctSplitVotes refBStyleId="b2" votes="3" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="cand-2" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="2">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="1" />
                            <CandidatePrecinctSplitVotes refBStyleId="b2" votes="1" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="und" type="UNDERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="1">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="1" />
                            <CandidatePrecinctSplitVotes refBStyleId="b2" votes="0" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &single_choice_config(&["mayor"]),
            &party_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        // Area A: candidate marks 4 + 1 = 5, plus 1 whole-ballot blank.
        assert!(csv.contains("PAPER,Area A,mayor,total_votes,,6"));
        assert!(csv.contains("PAPER,Area A,mayor,total_valid_votes,,6"));
        assert!(csv.contains("PAPER,Area A,mayor,census,,6"));
        assert!(csv.contains("PAPER,Area A,mayor,candidate_votes,cand-1,4"));
        assert!(csv.contains("PAPER,Area A,mayor,candidate_votes,cand-2,1"));
        // Area B: candidate marks 3 + 1 = 4, no blanks.
        assert!(csv.contains("PAPER,Area B,mayor,total_votes,,4"));
        assert!(csv.contains("PAPER,Area B,mayor,total_valid_votes,,4"));
        assert!(csv.contains("PAPER,Area B,mayor,census,,4"));
        assert!(csv.contains("PAPER,Area B,mayor,candidate_votes,cand-1,3"));
        assert!(csv.contains("PAPER,Area B,mayor,candidate_votes,cand-2,1"));
    }

    #[test]
    fn party_grouping_attributes_overvotes_and_undervotes_per_party() {
        // party_ballots_cast (9) is the authoritative total_votes; over/
        // under_votes are selection-slot counts recovered into ballot counts
        // via / max_votes. See
        // reports_a_validation_error_when_totals_do_not_reconcile for the
        // negative case of the reconciliation identity this relies on
        // (candidate marks + over_votes + under_votes == total_votes *
        // max_votes).
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="0" ballotsCast="9" blanksCast="1" />
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="9" blanksCast="1" />
                        <PrecinctParty partyId="1" ballotsCast="9" blanksCast="1">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="9" blanksCast="1" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                </PartyMap>
                <Contest altId1="contest-1">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="5">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="5" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="ovr" type="OVERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="1">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="1" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="und" type="UNDERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="3">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="3" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let contest_vote_config =
            HashMap::from([("contest-1".to_string(), ContestVoteConfig { max_votes: 1 })]);
        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &contest_vote_config,
            &party_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        // total_votes = party_ballots_cast(9); implicit_invalid =
        // over_votes(1) / max_votes(1) = 1; total_blank_votes =
        // under_votes(3) / max_votes(1) = 3; total_valid_votes = 9 - 1 = 8.
        // Reconciles: candidate marks(5) + over(1) + under(3) == 9 * 1.
        assert!(csv.contains("PAPER,Area A,contest-1,implicit_invalid,,1"));
        assert!(csv.contains("PAPER,Area A,contest-1,total_blank_votes,,3"));
        assert!(csv.contains("PAPER,Area A,contest-1,total_valid_votes,,8"));
        assert!(csv.contains("PAPER,Area A,contest-1,total_votes,,9"));
    }

    #[test]
    fn party_grouping_vote_for_n_contest_does_not_inflate_total_votes_past_census() {
        // 3 ballots, "vote for 3" — every voter fully used all 3 slots, so
        // candidate marks sum to 9 (3 ballots x 3 marks each). total_votes
        // must stay the ballot count (3), not the mark count (9) — this is
        // the exact regression this test guards: summing candidate marks
        // as if it were a ballot count broke every multi-select contest
        // downstream ("total votes must not exceed census").
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="0" ballotsCast="3" blanksCast="0" />
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="3" blanksCast="0" />
                        <PrecinctParty partyId="1" ballotsCast="3" blanksCast="0">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="3" blanksCast="0" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                </PartyMap>
                <Contest altId1="council">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="3">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="3" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="cand-2" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="3">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="3" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                    <Candidate altId1="cand-3" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="3">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="3" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let contest_vote_config =
            HashMap::from([("council".to_string(), ContestVoteConfig { max_votes: 3 })]);
        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &contest_vote_config,
            &party_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        assert!(csv.contains("PAPER,Area A,council,total_votes,,3"));
        assert!(csv.contains("PAPER,Area A,council,total_valid_votes,,3"));
        assert!(csv.contains("PAPER,Area A,council,census,,3"));
        assert!(csv.contains("PAPER,Area A,council,candidate_votes,cand-1,3"));
    }

    #[test]
    fn party_grouping_errors_on_conflicting_ballot_style_party_mapping() {
        // Ballot style "b1" is claimed by both party 1 and party 2 — a
        // data-quality problem in the source file that would silently
        // misattribute votes if picked arbitrarily, so this is a hard
        // whole-file error rather than a per-contest validation error.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="0" ballotsCast="5" blanksCast="0" />
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="5" blanksCast="0" />
                        <PrecinctParty partyId="1" ballotsCast="3" blanksCast="0">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="3" blanksCast="0" />
                        </PrecinctParty>
                        <PrecinctParty partyId="2" ballotsCast="2" blanksCast="0">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="2" blanksCast="0" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                    <Party id="2" name="Area B" />
                </PartyMap>
                <Contest altId1="contest-1">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="5">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="5" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let result = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &HashMap::new(),
            &party_area_names(),
        );

        let (csv, errors) = result.unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "ess_ballot_style_maps_to_multiple_parties");
        assert!(errors[0].message.contains("maps to both party id"));
        assert!(csv_has_no_rows(&csv));
    }

    #[test]
    fn party_grouping_refuses_when_party_data_exceeds_the_selected_reporting_group() {
        // The requested group 1 (Election Day) accounts for 9 ballots, but
        // the party data covers 12 — another reporting group (e.g. an
        // advance-voting channel) carries the other 3, and ES&S's
        // party-level data can't be scoped away from them (see
        // validate_party_data_matches_reporting_group's doc comment).
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="0" ballotsCast="12" blanksCast="0" />
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="9" blanksCast="0" />
                        <PrecinctParty partyId="1" ballotsCast="12" blanksCast="0">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="12" blanksCast="0" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                </PartyMap>
                <Contest altId1="contest-1">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="12">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="12" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let result = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &HashMap::new(),
            &party_area_names(),
        );

        let (csv, errors) = result.unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].code,
            "ess_party_data_not_scoped_to_reporting_group"
        );
        assert!(errors[0].message.contains("precinct id 'p1'"));
        assert!(errors[0]
            .message
            .contains("12 ballots across its PrecinctParty entries"));
        assert_eq!(
            errors[0].params.get("partyBallotsCast"),
            Some(&"12".to_string())
        );
        assert!(csv_has_no_rows(&csv));
    }

    #[test]
    fn party_grouping_refuses_party_overage_even_without_a_total_votes_group() {
        // Same inconsistency as above, but with no reportingGroupId="0"
        // present at all — the shape produced by trimming a file down to a
        // single reporting group. An earlier version of this guard compared
        // group 0 against the selected group and so became a silent no-op on
        // exactly these files, letting another channel's ballots through.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="0" blanksCast="0" />
                        <PrecinctParty partyId="1" ballotsCast="12" blanksCast="0">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="12" blanksCast="0" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                </PartyMap>
                <Contest altId1="contest-1">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="12">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="12" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let result = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &HashMap::new(),
            &party_area_names(),
        );

        let (csv, errors) = result.unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].code,
            "ess_party_data_not_scoped_to_reporting_group"
        );
        assert!(errors[0].message.contains("precinct id 'p1'"));
        assert!(errors[0]
            .message
            .contains("versus 0 in reporting group '1'"));
        assert!(csv_has_no_rows(&csv));
    }

    #[test]
    fn party_grouping_refuses_a_file_with_no_party_data_at_all() {
        // Party grouping is impossible without PrecinctParty data. This
        // fails once, up front, instead of once per contest further down
        // with a confusing "ballot style not present" message.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="9" blanksCast="0" />
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                </PartyMap>
                <Contest altId1="contest-1">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="9">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="9" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let result = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &HashMap::new(),
            &party_area_names(),
        );

        let (csv, errors) = result.unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].code,
            "ess_party_data_not_scoped_to_reporting_group"
        );
        assert!(errors[0]
            .message
            .contains("0 ballots across its PrecinctParty entries"));
        assert!(csv_has_no_rows(&csv));
    }

    #[test]
    fn party_grouping_errors_when_one_party_owns_two_ballot_styles() {
        // total_votes comes from <PrecinctParty ballotsCast>, which is only
        // a valid per-contest ballot count when every contest on the party's
        // ballot is on all of it — i.e. one ballot style per party. Two
        // styles could hide a contest that appears on only one of them, so
        // this is refused rather than silently inflating totals.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="10" blanksCast="0" />
                        <PrecinctParty partyId="1" ballotsCast="10" blanksCast="0">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="6" blanksCast="0" />
                            <PrecinctPartySplit refBStyleId="b2" ballotsCast="4" blanksCast="0" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                </PartyMap>
                <Contest altId1="contest-1">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="10">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="6" />
                            <CandidatePrecinctSplitVotes refBStyleId="b2" votes="4" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let result = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &HashMap::new(),
            &party_area_names(),
        );

        let (csv, errors) = result.unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "ess_party_maps_to_multiple_ballot_styles");
        assert!(errors[0].message.contains("maps to both ballot style id"));
        assert!(csv_has_no_rows(&csv));
    }

    #[test]
    fn party_grouping_succeeds_when_file_reports_only_the_selected_group() {
        // No reportingGroupId="0" ("Total Votes") at all — the file simply
        // doesn't report any other channel independently. The guard keys off
        // the party data rather than the Total Votes group, so this is
        // accepted on its own merits (party ballots 9 == group 1's 9), not
        // because a missing group is waved through.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="9" blanksCast="0" />
                        <PrecinctParty partyId="1" ballotsCast="9" blanksCast="0">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="9" blanksCast="0" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                </PartyMap>
                <Contest altId1="contest-1">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="9">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="9" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &single_choice_config(&["contest-1"]),
            &party_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        assert!(csv.contains("PAPER,Area A,contest-1,total_votes,,9"));
    }

    #[test]
    fn party_grouping_skips_contest_missing_party_split_data_but_converts_rest() {
        // contest-1 has no CandidatePrecinctSplitVotes at all (only the
        // bare precinct-level total) — a structural gap scoped to that one
        // contest, so it's reported as a validation error and skipped,
        // while contest-2 (which does carry the split) still converts.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="0" ballotsCast="5" blanksCast="0" />
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="5" blanksCast="0" />
                        <PrecinctParty partyId="1" ballotsCast="5" blanksCast="0">
                            <PrecinctPartySplit refBStyleId="b1" ballotsCast="5" blanksCast="0" />
                        </PrecinctParty>
                    </Precinct>
                </JurisdictionMap>
                <PartyMap>
                    <Party id="1" name="Area A" />
                </PartyMap>
                <Contest altId1="contest-1">
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="5" />
                    </Candidate>
                </Contest>
                <Contest altId1="contest-2">
                    <Candidate altId1="cand-2" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="5">
                            <CandidatePrecinctSplitVotes refBStyleId="b1" votes="5" />
                        </CandidatePrecinctVotes>
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &single_choice_config(&["contest-1", "contest-2"]),
            &party_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].contest_external_id, Some("contest-1".to_string()));
        assert!(!csv.contains("contest-1"));
        assert!(csv.contains("PAPER,Area A,contest-2,candidate_votes,cand-2,5"));
    }
}
