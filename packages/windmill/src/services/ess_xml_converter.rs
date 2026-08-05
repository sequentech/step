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

/// Which ES&S element supplies a tally sheet's `area_name`. Private to this
/// module on purpose: it describes one vendor's file layout, not a product
/// concept, and it is *detected* from the election event's configured Area
/// names rather than chosen by a caller — see `detect_area_grouping`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AreaGrouping {
    /// `<Precinct name>` — the physical polling place.
    Precinct,
    /// `<Party name>` — e.g. a school-support group, for events whose Areas
    /// are configured as ballot-style groups rather than polling places.
    Party,
}

impl AreaGrouping {
    /// Value recorded under `ESS_AREA_GROUPING_ANNOTATION_KEY`.
    fn as_str(self) -> &'static str {
        match self {
            AreaGrouping::Precinct => "PRECINCT",
            AreaGrouping::Party => "PARTY",
        }
    }
}

/// What a conversion produced: the canonical CSV, any validation errors, and
/// which ES&S element the area names were taken from (recorded on the import
/// for audit — see `ESS_AREA_GROUPING_ANNOTATION_KEY`).
#[derive(Debug, Clone)]
pub struct EssConversion {
    pub canonical_csv: Vec<u8>,
    pub validation_errors: Vec<TallySheetImportValidationError>,
    pub area_grouping: &'static str,
}

/// The STEP contest config this converter needs — `min_votes` decides
/// whether undervoting can invalidate a ballot; `max_votes` is the "vote
/// for N" bound used to check ES&S's reported totals reconcile exactly.
/// Missing/non-positive values default conservatively (`min_votes` to `0`,
/// `max_votes` to `1`, i.e. single-choice).
#[derive(Debug, Clone, Copy, Default)]
pub struct ContestVoteConfig {
    pub min_votes: i64,
    pub max_votes: i64,
}

#[derive(Debug, Clone, Default)]
struct ContestPrecinctTotals {
    /// Ballots cast across the whole precinct (every contest/ballot style
    /// on it) — always available, used only as the `census` upper bound.
    precinct_ballots_cast: u64,
    /// Ballots cast specifically for *this* contest in this precinct, when
    /// the XML variant reports it at that granularity (the
    /// `ContestReportingGroupVotes` variant does). `None` for the
    /// candidate-reporting-group variant: its `ballotsCast` lives on the
    /// `<Precinct>` element, shared across every contest on the ballot, so
    /// it isn't a valid `total_votes` for a contest that only appears on
    /// some ballot styles within that precinct (e.g. a ward- or
    /// school-board-specific race) — see the doc comment on
    /// `convert_ess_enhanced_xml_to_csv` for how each case is handled.
    contest_ballots_cast: Option<u64>,
    over_votes: u64,
    under_votes: u64,
    blank_votes: u64,
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

/// A candidate's votes keyed by precinct id (precinct grouping) or by party
/// id (party grouping) — same shape either way, so both grouping modes
/// share this struct.
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

/// Builds the validation error for a file whose area names match none of
/// the election event's Areas. Includes a few names from each side, since
/// the fix is almost always "these two lists were meant to be the same".
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
            "None of this file's area names match the election event's areas, so no tally sheet could be assigned to an area. The event's areas are [{configured_sample}]; the file's precincts are [{precinct_sample}] and its parties are [{party_sample}]"
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
    if over_votes % max_votes == 0 {
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
/// the precinct's whole-ballot blank count (`blanksCast`, read from
/// `PrecinctReportingGroup` — see `precinct_blanks_cast_by_id`). Every
/// ballot that's blank on the whole ballot is necessarily blank in this
/// contest too, so `total_blank_votes` can never be smaller — a violation
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

/// Resolves a single contest's precinct totals and candidate votes,
/// regardless of which of the two ES&S XML variants it uses.
fn resolve_contest_data(
    contest: Node<'_, '_>,
    document: &Document<'_>,
    reporting_group_id: &str,
) -> Result<(BTreeMap<String, ContestPrecinctTotals>, Vec<CandidateVotes>)> {
    if contest
        .children()
        .any(|node| node.has_tag_name("ContestReportingGroup"))
    {
        Ok((
            contest_totals_by_precinct(contest, reporting_group_id)?,
            normal_candidate_votes(contest)?,
        ))
    } else {
        candidate_reporting_group_contest_data(contest, document, reporting_group_id)
    }
}

/// Converts using the default reporting group and the default area
/// grouping.
///
/// This is the offline entry point (`step-cli`): with no election event
/// there are no configured Area names to match the file against, so the
/// grouping can't be detected and precinct grouping — the vendor-neutral
/// reading of an ES&S file — is used. Anything running with an election
/// event calls `convert_ess_enhanced_xml_to_csv_for_reporting_group`, which
/// detects it.
#[instrument(skip_all, err)]
pub fn convert_ess_enhanced_xml_to_csv(
    xml_bytes: &[u8],
    selected_channel: VotingChannel,
    contest_vote_config: &HashMap<String, ContestVoteConfig>,
) -> Result<(Vec<u8>, Vec<TallySheetImportValidationError>)> {
    let xml = std::str::from_utf8(xml_bytes).context("ES&S XML import must be valid UTF-8")?;
    let document = Document::parse(xml).context("Invalid ES&S Enhanced XML")?;
    convert_precinct_grouped(
        &document,
        selected_channel,
        DEFAULT_IMPORT_REPORTING_GROUP_ID,
        contest_vote_config,
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
/// `contest_vote_config` maps a contest's external id to its `min_votes`/
/// `max_votes` (STEP contest config); a contest missing from the map uses
/// the defaults documented on `ContestVoteConfig`. `min_votes` decides
/// whether undervoting can invalidate a ballot — see the comment above
/// `implicit_invalid` below. `max_votes` is used to check ES&S's reported
/// totals reconcile exactly — see the comment above `check_vote_reconciliation`.
///
/// `total_votes`/`total_valid_votes` are derived differently depending on
/// the XML variant. The `ContestReportingGroupVotes` variant reports
/// `ballotsCast` per contest *and* precinct, so it's used directly. The
/// candidate-reporting-group variant only reports `ballotsCast` per
/// precinct (shared across every contest on the ballot), which isn't a
/// valid ballot count for a contest that doesn't appear on every ballot
/// style in that precinct (e.g. a ward- or school-board-specific race) —
/// for that variant, `total_valid_votes` is derived from candidate marks
/// plus blank votes instead, the only figures actually scoped to this
/// contest and precinct. `census` always uses the precinct-wide ballots
/// cast, regardless of variant.
///
/// Where each tally sheet's `area_name` comes from is **detected**, not
/// configured: `configured_area_names` is the election event's Area names,
/// and whichever ES&S element matches them supplies the area names — see
/// `detect_area_grouping`.
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

    let area_grouping = match detect_area_grouping(&document, configured_area_names)? {
        Ok(area_grouping) => area_grouping,
        // Nothing in the file lines up with the event's Areas, so every row
        // this produced would fail to resolve. Report that once, up front,
        // instead of one "Area not found" per contest afterwards.
        Err(error) => {
            return Ok(EssConversion {
                canonical_csv: empty_canonical_csv()?,
                validation_errors: vec![error],
                area_grouping: AreaGrouping::Precinct.as_str(),
            });
        }
    };

    let (canonical_csv, validation_errors) = match area_grouping {
        AreaGrouping::Precinct => convert_precinct_grouped(
            &document,
            selected_channel,
            reporting_group_id,
            contest_vote_config,
        ),
        AreaGrouping::Party => convert_party_grouped(
            &document,
            selected_channel,
            reporting_group_id,
            contest_vote_config,
        ),
    }?;

    Ok(EssConversion {
        canonical_csv,
        validation_errors,
        area_grouping: area_grouping.as_str(),
    })
}

/// Picks the ES&S element that supplies area names, by seeing which one the
/// election event's Areas are actually named after. Downstream resolution
/// (`get_area_by_name`) is an exact match, so this compares exactly too.
///
/// Whichever element matches more configured Areas wins. A tie goes to
/// precinct grouping: it's the vendor-neutral reading of an ES&S file, and
/// party grouping carries extra preconditions on top. Matching *nothing* is
/// not a hard failure but `Ok(Err(..))` — a normal validation error, because
/// it means the file and the event disagree, which is exactly the kind of
/// data problem the import UI exists to show.
#[instrument(skip_all, err)]
#[allow(clippy::type_complexity)]
fn detect_area_grouping(
    document: &Document<'_>,
    configured_area_names: &HashSet<String>,
) -> Result<std::result::Result<AreaGrouping, TallySheetImportValidationError>> {
    let precinct_names = precinct_names_by_id(document)?;
    let party_names = party_names_by_id(document)?;

    let matches = |names: &HashMap<String, String>| -> usize {
        names
            .values()
            .filter(|name| configured_area_names.contains(*name))
            .count()
    };
    let precinct_matches = matches(&precinct_names);
    let party_matches = matches(&party_names);

    if precinct_matches == 0 && party_matches == 0 {
        return Ok(Err(area_detection_error(
            &precinct_names,
            &party_names,
            configured_area_names,
        )));
    }

    Ok(Ok(if party_matches > precinct_matches {
        AreaGrouping::Party
    } else {
        AreaGrouping::Precinct
    }))
}

/// The canonical CSV header with no data rows, for a conversion refused
/// before any contest could be read.
fn empty_canonical_csv() -> Result<Vec<u8>> {
    let mut writer = Writer::from_writer(Vec::new());
    write_canonical_csv_header(&mut writer)?;
    writer.into_inner().map_err(|err| anyhow!(err))
}

/// Precinct-grouped ES&S conversion — `area_name` is each `<Precinct
/// name>`. This is the original/default conversion behavior; see the doc
/// comment on `convert_ess_enhanced_xml_to_csv_for_reporting_group` for the
/// full field-derivation rules.
fn convert_precinct_grouped(
    document: &Document<'_>,
    selected_channel: VotingChannel,
    reporting_group_id: &str,
    contest_vote_config: &HashMap<String, ContestVoteConfig>,
) -> Result<(Vec<u8>, Vec<TallySheetImportValidationError>)> {
    let precinct_names = precinct_names_by_id(document)?;
    let precinct_blanks_cast = precinct_blanks_cast_by_id(document, reporting_group_id);
    let mut writer = Writer::from_writer(Vec::new());
    let mut validation_errors = Vec::new();

    write_canonical_csv_header(&mut writer)?;

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

        let (totals_by_precinct, candidates) =
            match resolve_contest_data(contest, document, reporting_group_id) {
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

        let vote_config = contest_vote_config
            .get(&contest_external_id)
            .copied()
            .unwrap_or_default();
        let min_votes = vote_config.min_votes;
        let max_votes = vote_config.max_votes.max(1) as u64;

        for (precinct_id, totals) in totals_by_precinct {
            let Some(area_name) = precinct_names.get(&precinct_id) else {
                validation_errors.push(xml_error(
                    &selected_channel,
                    Some(&contest_external_id),
                    format!(
                        "Contest references precinct id '{}' not present in JurisdictionMap",
                        precinct_id
                    ),
                ));
                continue;
            };

            // For the ContestReportingGroupVotes variant (contest_ballots_cast
            // is Some), ES&S's own `blankVotes` is not a genuine blank-ballot
            // count — it's exactly `overVotes + underVotes` (confirmed both
            // by the EVS SOP's field description and empirically, with no
            // exceptions). `underVotes` is a *selection-slot* count, not a
            // ballot count (same issue as `overVotes` — see
            // check_over_votes_divisible's doc comment): it sums unused
            // slots from both genuinely blank ballots (which contribute
            // their whole max_votes allotment) and ballots with a valid
            // partial selection (which contribute their remaining unused
            // slots too), so `underVotes / max_votes` is only an
            // *upper-bound approximation* of the blank-ballot count, exact
            // only when every under-filled ballot is entirely blank — no
            // field in ES&S's aggregate XML distinguishes the two cases.
            // `check_blank_votes_at_least_precinct_minimum` below
            // sanity-checks this approximation against a genuine (if
            // precinct-wide) lower bound. The candidate-reporting-group
            // variant doesn't have this problem — its blank figure comes
            // directly from the precinct's own `blanksCast`, a genuine (if
            // precinct-wide) blank-ballot count — so it keeps using that.
            let total_blank_votes = if totals.contest_ballots_cast.is_some() {
                totals.under_votes / max_votes
            } else {
                totals.blank_votes
            };
            // Overvoting a "vote for N" contest always spoils that contest
            // on the ballot, regardless of over_vote_policy — see
            // sequent_core::ballot_codec::checker::check_over_vote_policy,
            // whose policy branches only change alert UI copy; the
            // invalid_errors push (the actual invalidity decision) is
            // unconditional there.
            //
            // ES&S's overVotes is a *selection-slot* count, not a ballot
            // count (see check_over_votes_divisible's doc comment). For the
            // ContestReportingGroupVotes variant, total_votes below is a
            // genuine ballot count from ES&S, so overVotes must be divided
            // by max_votes to recover an overvoted-*ballot* count —
            // otherwise implicit_invalid could exceed total_votes and make
            // total_valid_votes underflow to 0. The other variant derives
            // total_votes from candidate marks/blank votes instead (see the
            // total_votes/total_valid_votes match below), so it isn't
            // exposed to that underflow and keeps using the raw slot count.
            let mut implicit_invalid = if totals.contest_ballots_cast.is_some() {
                totals.over_votes / max_votes
            } else {
                totals.over_votes
            };
            // Undervoting only invalidates a ballot when it selects fewer
            // candidates than min_votes requires — see
            // check_under_vote_policy's comment: falling short of
            // min_votes "is an invalid vote no matter what" the
            // under_vote_policy is; under_vote_policy itself never
            // invalidates a ballot on its own. ES&S's aggregate under_votes
            // count doesn't distinguish "fell short of min_votes" from any
            // other undervote, so it can't be attributed precisely — this
            // is a best-effort approximation, and only applies to the
            // candidate-reporting-group variant, where total_blank_votes is
            // an independent, genuinely ballot-scoped figure (blanksCast)
            // that under_votes (a slot count) can meaningfully overlap
            // with. The ContestReportingGroupVotes variant's
            // total_blank_votes above is itself already derived from
            // under_votes, so subtracting it back out here wouldn't be a
            // meaningful correction (the two figures use different units —
            // see the comment above total_blank_votes).
            if min_votes > 0 && totals.contest_ballots_cast.is_none() {
                implicit_invalid += totals.under_votes.saturating_sub(total_blank_votes);
            }
            let explicit_invalid = 0;
            let total_invalid = implicit_invalid + explicit_invalid;
            let candidate_votes_sum = candidates
                .iter()
                .map(|candidate| {
                    candidate
                        .votes_by_key
                        .get(&precinct_id)
                        .copied()
                        .unwrap_or(0)
                })
                .sum::<u64>();
            let (total_votes, total_valid_votes) = match totals.contest_ballots_cast {
                // A single ballot can legitimately carry more than one
                // candidate mark (e.g. "vote for N" contests), so
                // total_votes comes from ES&S's own per-contest
                // ballots-cast figure rather than being derived from the
                // sum of candidate marks.
                Some(contest_ballots_cast) => {
                    let total_votes = contest_ballots_cast;
                    let total_valid_votes = total_votes.saturating_sub(total_invalid);
                    (total_votes, total_valid_votes)
                }
                // No ballots-cast figure is scoped to this contest and
                // precinct (candidate-reporting-group variant) — the only
                // figures that are scoped that way are the candidate marks
                // and blank votes, so total_valid_votes is derived from
                // those instead, same as for a single-choice contest.
                None => {
                    let total_valid_votes = candidate_votes_sum + total_blank_votes;
                    let total_votes = total_valid_votes + total_invalid;
                    (total_votes, total_valid_votes)
                }
            };

            write_scalar_row(
                &mut writer,
                &selected_channel,
                area_name,
                &contest_external_id,
                "total_votes",
                total_votes,
            )?;
            write_scalar_row(
                &mut writer,
                &selected_channel,
                area_name,
                &contest_external_id,
                "total_valid_votes",
                total_valid_votes,
            )?;
            write_scalar_row(
                &mut writer,
                &selected_channel,
                area_name,
                &contest_external_id,
                "implicit_invalid",
                implicit_invalid,
            )?;
            write_scalar_row(
                &mut writer,
                &selected_channel,
                area_name,
                &contest_external_id,
                "explicit_invalid",
                explicit_invalid,
            )?;
            write_scalar_row(
                &mut writer,
                &selected_channel,
                area_name,
                &contest_external_id,
                "total_blank_votes",
                total_blank_votes,
            )?;
            write_scalar_row(
                &mut writer,
                &selected_channel,
                area_name,
                &contest_external_id,
                "census",
                totals.precinct_ballots_cast,
            )?;

            // Only meaningful when total_votes is ES&S's own per-contest
            // ballots-cast figure (not derived from candidate marks) — see
            // check_vote_reconciliation's doc comment for the identity
            // being checked.
            if totals.contest_ballots_cast.is_some() {
                write_scalar_row(
                    &mut writer,
                    &selected_channel,
                    area_name,
                    &contest_external_id,
                    "over_votes",
                    totals.over_votes,
                )?;
                write_scalar_row(
                    &mut writer,
                    &selected_channel,
                    area_name,
                    &contest_external_id,
                    "under_votes",
                    totals.under_votes,
                )?;
                if let Some(error) = check_over_votes_divisible(
                    &selected_channel,
                    area_name,
                    &contest_external_id,
                    totals.over_votes,
                    max_votes,
                ) {
                    validation_errors.push(error);
                }
                if let Some(error) = check_vote_reconciliation(
                    &selected_channel,
                    area_name,
                    &contest_external_id,
                    candidate_votes_sum,
                    totals.over_votes,
                    totals.under_votes,
                    total_votes,
                    max_votes,
                ) {
                    validation_errors.push(error);
                }
                if let Some(&precinct_blanks_cast_value) = precinct_blanks_cast.get(&precinct_id) {
                    if let Some(error) = check_blank_votes_at_least_precinct_minimum(
                        &selected_channel,
                        area_name,
                        &contest_external_id,
                        total_blank_votes,
                        precinct_blanks_cast_value,
                    ) {
                        validation_errors.push(error);
                    }
                }
            }

            for candidate in &candidates {
                writer.write_record([
                    selected_channel.to_string(),
                    area_name.to_string(),
                    contest_external_id.clone(),
                    "candidate_votes".to_string(),
                    candidate.external_id.clone(),
                    candidate
                        .votes_by_key
                        .get(&precinct_id)
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
/// race on every ballot). Unlike `convert_precinct_grouped`'s candidate-
/// reporting-group branch, party grouping *does* have an authoritative
/// ballot count: a party maps to exactly one ballot style (see
/// `ballot_style_to_party_id`), so every contest on that ballot style is on
/// every one of that party's ballots — `<PrecinctParty ballotsCast>` is
/// therefore this contest's genuine ballot count too, not just a whole-
/// ballot upper bound. This follows `convert_precinct_grouped`'s *other*
/// (`ContestReportingGroupVotes`) branch instead: `total_votes` is that
/// authoritative figure, `over_votes`/`under_votes` are selection-slot
/// counts recovered into ballot counts via `/ max_votes` (never derived
/// from summing candidate marks, which can legitimately exceed the ballot
/// count on a "vote for N" contest), and `check_vote_reconciliation`/
/// `check_over_votes_divisible`/`check_blank_votes_at_least_precinct_minimum`
/// all run, same as that branch.
///
/// Neither `CandidatePrecinctSplitVotes` nor `PrecinctParty` carry a
/// `reportingGroupId` of their own, so this data can't be scoped to a
/// specific reporting group (e.g. election-day vs. absentee) the way
/// `convert_precinct_grouped` can — `validate_party_data_matches_reporting_group`
/// refuses the conversion up front, unless each precinct's party data adds
/// up to exactly the requested group, rather than silently blending every
/// channel's ballots into whichever one was requested.
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
fn convert_party_grouped(
    document: &Document<'_>,
    selected_channel: VotingChannel,
    reporting_group_id: &str,
    contest_vote_config: &HashMap<String, ContestVoteConfig>,
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

        let vote_config = contest_vote_config
            .get(&contest_external_id)
            .copied()
            .unwrap_or_default();
        let max_votes = vote_config.max_votes.max(1) as u64;

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

            // Unlike convert_precinct_grouped's candidate-reporting-group
            // branch, party grouping *does* have an authoritative ballot
            // count here: `party_ballots_cast` is genuinely this contest's
            // ballot count, not just an upper bound, because a party maps
            // to exactly one ballot style (see ballot_style_to_party_id) —
            // every contest on that ballot style is on every one of that
            // party's ballots, with no partial appearance. So this follows
            // convert_precinct_grouped's *other* (ContestReportingGroupVotes)
            // branch instead: total_votes is the authoritative figure, not
            // derived from candidate marks — candidate_votes_sum can
            // legitimately exceed the ballot count on a "vote for N"
            // contest, so it must never be treated as a ballot count (the
            // bug this replaced: multi-select contests were tripping the
            // "total votes must not exceed census" check downstream).
            // over_votes/under_votes are selection-slot counts here too
            // (from the OVERVOTES/UNDERVOTES pseudo-candidates' party-
            // attributed marks), so they need the same /max_votes recovery
            // — see check_over_votes_divisible's doc comment.
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

/// Reads each precinct's whole-ballot blank count (`blanksCast`) from its
/// `PrecinctReportingGroup` matching `reporting_group_id`, when present.
/// This is a genuine, if precinct-wide (not per-contest), blank-*ballot*
/// count — used only as a lower-bound sanity check on a contest's derived
/// `total_blank_votes` (see `check_blank_votes_at_least_precinct_minimum`),
/// never as the figure itself. Missing data (e.g. a precinct with no
/// matching reporting group) is treated as "unknown" rather than a hard
/// failure, since it's only needed for this optional check.
#[instrument(skip_all)]
fn precinct_blanks_cast_by_id(
    document: &Document<'_>,
    reporting_group_id: &str,
) -> HashMap<String, u64> {
    let mut blanks_cast_by_precinct = HashMap::new();
    for precinct in document
        .descendants()
        .filter(|node| node.has_tag_name("Precinct"))
    {
        let Some(precinct_id) = precinct.attribute("id") else {
            continue;
        };
        for group in precinct
            .children()
            .filter(|node| node.has_tag_name("PrecinctReportingGroup"))
        {
            if group.attribute("reportingGroupId") != Some(reporting_group_id) {
                continue;
            }
            if let Some(blanks_cast) = group
                .attribute("blanksCast")
                .and_then(|value| value.parse::<u64>().ok())
            {
                blanks_cast_by_precinct.insert(precinct_id.to_string(), blanks_cast);
            }
        }
    }
    blanks_cast_by_precinct
}

// Keyed by a BTreeMap so the canonical CSV rows are emitted in a stable
// precinct order and the stored canonical_csv_sha256 is reproducible.
#[instrument(skip_all, err)]
fn contest_totals_by_precinct(
    contest: Node<'_, '_>,
    reporting_group_id: &str,
) -> Result<BTreeMap<String, ContestPrecinctTotals>> {
    let mut totals_by_precinct: BTreeMap<String, ContestPrecinctTotals> = BTreeMap::new();
    for group in contest
        .children()
        .filter(|node| node.has_tag_name("ContestReportingGroup"))
    {
        let group_reporting_group_id =
            required_attr(group, "reportingGroupId", "ContestReportingGroup")?;
        if group_reporting_group_id != reporting_group_id {
            continue;
        }
        for votes in group
            .children()
            .filter(|node| node.has_tag_name("ContestReportingGroupVotes"))
        {
            let precinct_id = required_attr(votes, "refPrecinctId", "ContestReportingGroupVotes")?;
            let entry = totals_by_precinct.entry(precinct_id).or_default();
            let ballots_cast = parse_u64_attr(votes, "ballotsCast", "ContestReportingGroupVotes")?;
            entry.precinct_ballots_cast += ballots_cast;
            entry.contest_ballots_cast =
                Some(entry.contest_ballots_cast.unwrap_or(0) + ballots_cast);
            entry.over_votes += parse_u64_attr(votes, "overVotes", "ContestReportingGroupVotes")?;
            entry.under_votes += parse_u64_attr(votes, "underVotes", "ContestReportingGroupVotes")?;
            entry.blank_votes += parse_u64_attr(votes, "blankVotes", "ContestReportingGroupVotes")?;
        }
    }
    Ok(totals_by_precinct)
}

#[instrument(skip_all, err)]
fn normal_candidate_votes(contest: Node<'_, '_>) -> Result<Vec<CandidateVotes>> {
    let mut candidates = Vec::new();
    for candidate in contest
        .children()
        .filter(|node| node.has_tag_name("Candidate"))
    {
        let candidate_type = candidate.attribute("type").unwrap_or("NORMAL");
        if candidate_type == "OVERVOTES" || candidate_type == "UNDERVOTES" {
            continue;
        }
        let external_id = required_attr(candidate, "altId1", "Candidate")?;
        if external_id.trim().is_empty() {
            return Err(anyhow!("Candidate is missing altId1 import id"));
        }
        let mut votes_by_key = HashMap::new();
        for votes in candidate
            .children()
            .filter(|node| node.has_tag_name("CandidatePrecinctVotes"))
        {
            let precinct_id = required_attr(votes, "refPrecinctId", "CandidatePrecinctVotes")?;
            let vote_count = parse_u64_attr(votes, "votes", "CandidatePrecinctVotes")?;
            if votes_by_key
                .insert(precinct_id.clone(), vote_count)
                .is_some()
            {
                return Err(anyhow!(
                    "Duplicate CandidatePrecinctVotes for candidate altId1 '{}' and precinct id '{}'",
                    external_id,
                    precinct_id
                ));
            }
        }
        candidates.push(CandidateVotes {
            external_id,
            votes_by_key,
        });
    }
    Ok(candidates)
}

#[instrument(skip_all, err)]
fn candidate_reporting_group_contest_data(
    contest: Node<'_, '_>,
    document: &Document<'_>,
    reporting_group_id: &str,
) -> Result<(BTreeMap<String, ContestPrecinctTotals>, Vec<CandidateVotes>)> {
    let precinct_totals =
        precinct_reporting_group_totals_by_precinct(document, reporting_group_id)?;
    let mut totals_by_precinct: BTreeMap<String, ContestPrecinctTotals> = BTreeMap::new();
    let mut candidates = Vec::new();

    for candidate in contest
        .children()
        .filter(|node| node.has_tag_name("Candidate"))
    {
        let candidate_type = candidate.attribute("type").unwrap_or("NORMAL");
        let votes_by_precinct =
            candidate_reporting_group_votes_by_precinct(candidate, reporting_group_id)?;

        for precinct_id in votes_by_precinct.keys() {
            if totals_by_precinct.contains_key(precinct_id) {
                continue;
            }
            let Some(precinct_totals_entry) = precinct_totals.get(precinct_id) else {
                return Err(anyhow!(
                    "CandidateReportingGroupPrecinct references precinct id '{}' not present in PrecinctReportingGroup reportingGroupId={} data",
                    precinct_id,
                    reporting_group_id
                ));
            };
            totals_by_precinct.insert(precinct_id.clone(), precinct_totals_entry.clone());
        }

        if candidate_type == "OVERVOTES" {
            for (precinct_id, votes) in votes_by_precinct {
                totals_by_precinct
                    .entry(precinct_id)
                    .or_default()
                    .over_votes += votes;
            }
            continue;
        }

        if candidate_type == "UNDERVOTES" {
            for (precinct_id, votes) in votes_by_precinct {
                totals_by_precinct
                    .entry(precinct_id)
                    .or_default()
                    .under_votes += votes;
            }
            continue;
        }

        let external_id = required_attr(candidate, "altId1", "Candidate")?;
        if external_id.trim().is_empty() {
            return Err(anyhow!("Candidate is missing altId1 import id"));
        }

        candidates.push(CandidateVotes {
            external_id,
            votes_by_key: votes_by_precinct,
        });
    }

    if totals_by_precinct.is_empty() {
        return Err(anyhow!(
            "Contest is missing CandidateReportingGroup data for reportingGroupId={}",
            reporting_group_id
        ));
    }

    Ok((totals_by_precinct, candidates))
}

#[instrument(skip_all, err)]
fn precinct_reporting_group_totals_by_precinct(
    document: &Document<'_>,
    reporting_group_id: &str,
) -> Result<HashMap<String, ContestPrecinctTotals>> {
    let mut totals_by_precinct: HashMap<String, ContestPrecinctTotals> = HashMap::new();

    for precinct in document
        .descendants()
        .filter(|node| node.has_tag_name("Precinct"))
    {
        let precinct_id = required_attr(precinct, "id", "Precinct")?;
        let mut found_import_group = false;

        for reporting_group in precinct
            .children()
            .filter(|node| node.has_tag_name("PrecinctReportingGroup"))
        {
            let group_reporting_group_id = required_attr(
                reporting_group,
                "reportingGroupId",
                "PrecinctReportingGroup",
            )?;
            if group_reporting_group_id != reporting_group_id {
                continue;
            }

            found_import_group = true;
            let entry = totals_by_precinct.entry(precinct_id.clone()).or_default();
            entry.precinct_ballots_cast +=
                parse_u64_attr(reporting_group, "ballotsCast", "PrecinctReportingGroup")?;
            entry.blank_votes +=
                parse_u64_attr(reporting_group, "blanksCast", "PrecinctReportingGroup")?;
        }

        if !found_import_group {
            continue;
        }
    }

    Ok(totals_by_precinct)
}

#[instrument(skip_all, err)]
fn candidate_reporting_group_votes_by_precinct(
    candidate: Node<'_, '_>,
    reporting_group_id: &str,
) -> Result<HashMap<String, u64>> {
    let mut votes_by_precinct = HashMap::new();

    for reporting_group in candidate
        .children()
        .filter(|node| node.has_tag_name("CandidateReportingGroup"))
    {
        let group_reporting_group_id = required_attr(
            reporting_group,
            "reportingGroupId",
            "CandidateReportingGroup",
        )?;
        if group_reporting_group_id != reporting_group_id {
            continue;
        }

        for votes in reporting_group
            .children()
            .filter(|node| node.has_tag_name("CandidateReportingGroupPrecinct"))
        {
            let precinct_id =
                required_attr(votes, "refPrecinctId", "CandidateReportingGroupPrecinct")?;
            let vote_count = parse_u64_attr(votes, "votes", "CandidateReportingGroupPrecinct")?;
            if votes_by_precinct
                .insert(precinct_id.clone(), vote_count)
                .is_some()
            {
                return Err(anyhow!(
                    "Duplicate CandidateReportingGroupPrecinct for precinct id '{}' in reportingGroupId={}",
                    precinct_id,
                    reporting_group_id
                ));
            }
        }
    }

    Ok(votes_by_precinct)
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

    /// Area names matching the `<Precinct name>` values the precinct
    /// fixtures below use, so detection selects precinct grouping.
    fn precinct_area_names() -> HashSet<String> {
        ["Precinct 1", "Ward 1", "Ward 2"]
            .into_iter()
            .map(String::from)
            .collect()
    }

    /// Area names matching the `<Party name>` values the party fixtures
    /// below use, so detection selects party grouping.
    fn party_area_names() -> HashSet<String> {
        ["Area A", "Area B"].into_iter().map(String::from).collect()
    }

    /// Most tests only care about the CSV and the validation errors, not
    /// which grouping was detected (that is asserted on its own below).
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

    /// A file whose contests live on both precincts and parties, so which
    /// grouping is chosen depends purely on the configured Area names.
    const AMBIGUOUS_SHAPED_FILE: &[u8] = br#"
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
            AMBIGUOUS_SHAPED_FILE,
            VotingChannel::PAPER,
            DEFAULT_IMPORT_REPORTING_GROUP_ID,
            &HashMap::new(),
            area_names,
        )
        .unwrap()
    }

    #[test]
    fn detects_precinct_grouping_when_areas_are_named_after_precincts() {
        let conversion = convert_with_area_names(&precinct_area_names());

        assert_eq!(conversion.area_grouping, "PRECINCT");
        assert!(conversion.validation_errors.is_empty());
        let csv = String::from_utf8(conversion.canonical_csv).unwrap();
        assert!(csv.contains("PAPER,Precinct 1,contest-1,candidate_votes,cand-1,4"));
    }

    #[test]
    fn detects_party_grouping_when_areas_are_named_after_parties() {
        // Same file as above — only the event's Area names differ.
        let conversion = convert_with_area_names(&party_area_names());

        assert_eq!(conversion.area_grouping, "PARTY");
        assert!(conversion.validation_errors.is_empty());
        let csv = String::from_utf8(conversion.canonical_csv).unwrap();
        assert!(csv.contains("PAPER,Area A,contest-1,candidate_votes,cand-1,4"));
    }

    #[test]
    fn prefers_precinct_grouping_when_both_match_equally() {
        // Pathological event whose Areas happen to carry both names. Precinct
        // is the vendor-neutral reading, so it wins the tie.
        let both: HashSet<String> = ["Precinct 1", "Area A"]
            .into_iter()
            .map(String::from)
            .collect();

        assert_eq!(convert_with_area_names(&both).area_grouping, "PRECINCT");
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
        assert!(error.message.contains("Precinct 1"));
        assert!(error.message.contains("Area A"));
        assert!(error.message.contains("Somewhere Else"));
        assert!(csv_has_no_rows(&conversion.canonical_csv));
    }

    #[test]
    fn offline_conversion_without_an_election_event_is_precinct_grouped() {
        // step-cli has no election event, so no Area names to match against
        // and nothing to detect — it reads the file the vendor-neutral way.
        // This fixture has no party data at all, so a party reading would be
        // refused outright; converting proves precinct grouping was used.
        let (csv, errors) = convert_ess_enhanced_xml_to_csv(
            br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1" />
                </JurisdictionMap>
                <Contest altId1="contest-1">
                    <ContestReportingGroup reportingGroupId="1">
                        <ContestReportingGroupVotes refPrecinctId="p1" ballotsCast="4" overVotes="0" underVotes="0" blankVotes="0" />
                    </ContestReportingGroup>
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="4" />
                    </Candidate>
                </Contest>
            </ElectionReport>
            "#,
            VotingChannel::PAPER,
            &HashMap::new(),
        )
        .unwrap();

        assert!(errors.is_empty());
        let csv = String::from_utf8(csv).unwrap();
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_votes,,4"));
    }

    #[test]
    fn converts_enhanced_xml_to_overlap_safe_canonical_csv() {
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1" />
                </JurisdictionMap>
                <Contest altId1="contest-1">
                    <ContestReportingGroup reportingGroupId="1">
                        <ContestReportingGroupVotes refPrecinctId="p1" ballotsCast="20" overVotes="2" underVotes="5" blankVotes="4" />
                    </ContestReportingGroup>
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="10" />
                    </Candidate>
                    <Candidate altId1="cand-2" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="3" />
                    </Candidate>
                    <Candidate altId1="ignored-overvotes" type="OVERVOTES">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="2" />
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &HashMap::new(),
            &precinct_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        // min_votes defaults to 0 for a contest missing from the map, so
        // the 3 under-votes here never invalidate a ballot — only the 2
        // over-votes do. candidate_sum(13) + over(2) + under(5) ==
        // ballots_cast(20) * max_votes(1, the default), satisfying the
        // reconciliation check.
        assert!(csv.contains("PAPER,Precinct 1,contest-1,implicit_invalid,,2"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_blank_votes,,5"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_valid_votes,,18"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_votes,,20"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,candidate_votes,cand-1,10"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,over_votes,,2"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,under_votes,,5"));
        assert!(!csv.contains("ignored-overvotes"));
    }

    #[test]
    fn undervotes_only_count_as_invalid_when_contest_requires_a_minimum() {
        // The candidate-reporting-group variant, where total_blank_votes
        // (from the precinct's own blanksCast) and under_votes (from this
        // contest's UNDERVOTES pseudo-candidate) are independent figures,
        // so the overlap-safe subtraction has an observable effect. This
        // contest requires at least 1 selection (min_votes=1), so the
        // portion of under_votes beyond the precinct's blank ballots is
        // folded into implicit_invalid.
        let xml = br#"
            <Owner name="EVS Electionware Enhanced XML Results File version 1.3">
                <JurisdictionMap>
                    <Jurisdiction id="1" title="Jurisdiction">
                        <Precinct id="precinct-1" name="Precinct 1">
                            <PrecinctReportingGroup reportingGroupId="1" ballotsCast="20" blanksCast="4"/>
                        </Precinct>
                        <Contest id="contest-source-1" altId1="contest-1" title="Contest">
                            <Candidate id="candidate-source-a" type="NORMAL" altId1="cand-1" name="Candidate A">
                                <CandidateReportingGroup reportingGroupId="1" totalVotes="7">
                                    <CandidateReportingGroupPrecinct refPrecinctId="precinct-1" votes="7"/>
                                </CandidateReportingGroup>
                            </Candidate>
                            <Candidate id="candidate-source-b" type="NORMAL" altId1="cand-2" name="Candidate B">
                                <CandidateReportingGroup reportingGroupId="1" totalVotes="3">
                                    <CandidateReportingGroupPrecinct refPrecinctId="precinct-1" votes="3"/>
                                </CandidateReportingGroup>
                            </Candidate>
                            <Candidate id="source-overvotes" type="OVERVOTES" altId1="" name="OverVotes">
                                <CandidateReportingGroup reportingGroupId="1" totalVotes="2">
                                    <CandidateReportingGroupPrecinct refPrecinctId="precinct-1" votes="2"/>
                                </CandidateReportingGroup>
                            </Candidate>
                            <Candidate id="source-undervotes" type="UNDERVOTES" altId1="" name="UnderVotes">
                                <CandidateReportingGroup reportingGroupId="1" totalVotes="5">
                                    <CandidateReportingGroupPrecinct refPrecinctId="precinct-1" votes="5"/>
                                </CandidateReportingGroup>
                            </Candidate>
                        </Contest>
                    </Jurisdiction>
                </JurisdictionMap>
            </Owner>
        "#;

        let contest_vote_config = HashMap::from([(
            "contest-1".to_string(),
            ContestVoteConfig {
                min_votes: 1,
                max_votes: 1,
            },
        )]);
        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &contest_vote_config,
            &precinct_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        assert!(csv.contains("PAPER,Precinct 1,contest-1,implicit_invalid,,3"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_blank_votes,,4"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_valid_votes,,14"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_votes,,17"));
        // candidate-reporting-group variant: no per-contest ballots-cast,
        // so no over_votes/under_votes annotations are emitted either.
        assert!(!csv.contains("contest-1,over_votes"));
        assert!(!csv.contains("contest-1,under_votes"));
    }

    #[test]
    fn derives_totals_from_ballots_cast_not_candidate_marks() {
        // A "vote for 2" contest: candidate marks (15 + 12 = 27) legitimately
        // exceed ballots_cast (20) because each ballot can select up to 2
        // candidates. total_votes/total_valid_votes must still be derived
        // from ballots_cast, not from summing candidate marks. underVotes
        // (13) accounts for the rest of the 40 available slots (20 * 2)
        // not used by a candidate mark, satisfying the reconciliation
        // check: 27 + 0 + 13 == 20 * 2.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1" />
                </JurisdictionMap>
                <Contest altId1="contest-1">
                    <ContestReportingGroup reportingGroupId="1">
                        <ContestReportingGroupVotes refPrecinctId="p1" ballotsCast="20" overVotes="0" underVotes="13" blankVotes="13" />
                    </ContestReportingGroup>
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="15" />
                    </Candidate>
                    <Candidate altId1="cand-2" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="12" />
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let contest_vote_config = HashMap::from([(
            "contest-1".to_string(),
            ContestVoteConfig {
                min_votes: 0,
                max_votes: 2,
            },
        )]);
        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &contest_vote_config,
            &precinct_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_votes,,20"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_valid_votes,,20"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,candidate_votes,cand-1,15"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,candidate_votes,cand-2,12"));
    }

    #[test]
    fn converts_candidate_reporting_group_variant_to_canonical_csv() {
        let xml = br#"
            <Owner name="EVS Electionware Enhanced XML Results File version 1.3">
                <JurisdictionMap>
                    <Jurisdiction id="1" title="Jurisdiction">
                        <Precinct id="precinct-1" name="Precinct 1">
                            <PrecinctReportingGroup reportingGroupId="0" ballotsCast="100" blanksCast="6"/>
                            <PrecinctReportingGroup reportingGroupId="1" ballotsCast="100" blanksCast="6"/>
                        </Precinct>
                        <Contest id="contest-source-1" altId1="contest-1" title="Contest">
                            <Candidate id="candidate-source-a" type="NORMAL" altId1="cand-1" name="Candidate A">
                                <CandidateReportingGroup reportingGroupId="0" totalVotes="58">
                                    <CandidateReportingGroupPrecinct refPrecinctId="precinct-1" votes="58"/>
                                </CandidateReportingGroup>
                                <CandidateReportingGroup reportingGroupId="1" totalVotes="58">
                                    <CandidateReportingGroupPrecinct refPrecinctId="precinct-1" votes="58"/>
                                </CandidateReportingGroup>
                            </Candidate>
                            <Candidate id="candidate-source-b" type="NORMAL" altId1="cand-2" name="Candidate B">
                                <CandidateReportingGroup reportingGroupId="1" totalVotes="34">
                                    <CandidateReportingGroupPrecinct refPrecinctId="precinct-1" votes="34"/>
                                </CandidateReportingGroup>
                            </Candidate>
                            <Candidate id="source-overvotes" type="OVERVOTES" altId1="" name="OverVotes">
                                <CandidateReportingGroup reportingGroupId="1" totalVotes="2">
                                    <CandidateReportingGroupPrecinct refPrecinctId="precinct-1" votes="2"/>
                                </CandidateReportingGroup>
                            </Candidate>
                            <Candidate id="source-undervotes" type="UNDERVOTES" altId1="" name="UnderVotes">
                                <CandidateReportingGroup reportingGroupId="1" totalVotes="6">
                                    <CandidateReportingGroupPrecinct refPrecinctId="precinct-1" votes="6"/>
                                </CandidateReportingGroup>
                            </Candidate>
                        </Contest>
                    </Jurisdiction>
                </JurisdictionMap>
            </Owner>
        "#;

        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &HashMap::new(),
            &precinct_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        assert!(csv.contains("PAPER,Precinct 1,contest-1,candidate_votes,cand-1,58"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,candidate_votes,cand-2,34"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_blank_votes,,6"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,census,,100"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,implicit_invalid,,2"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_valid_votes,,98"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_votes,,100"));
    }

    #[test]
    fn derives_totals_from_marks_for_a_contest_not_on_every_ballot_style() {
        // The candidate-reporting-group variant's ballotsCast is reported
        // per precinct only (shared across every contest on the ballot),
        // not per contest. A contest that only appears on some ballot
        // styles in the precinct (e.g. a school-board trustee race) will
        // have far fewer candidate marks than the precinct's ballotsCast —
        // total_votes must come from those marks, not the precinct total.
        let xml = br#"
            <Owner name="EVS Electionware Enhanced XML Results File version 1.3">
                <JurisdictionMap>
                    <Jurisdiction id="1" title="Jurisdiction">
                        <Precinct id="precinct-1" name="Precinct 1">
                            <PrecinctReportingGroup reportingGroupId="1" ballotsCast="100" blanksCast="1"/>
                        </Precinct>
                        <Contest id="contest-source-1" altId1="contest-1" title="Contest">
                            <Candidate id="candidate-source-a" type="NORMAL" altId1="cand-1" name="Candidate A">
                                <CandidateReportingGroup reportingGroupId="1" totalVotes="5">
                                    <CandidateReportingGroupPrecinct refPrecinctId="precinct-1" votes="5"/>
                                </CandidateReportingGroup>
                            </Candidate>
                        </Contest>
                    </Jurisdiction>
                </JurisdictionMap>
            </Owner>
        "#;

        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &HashMap::new(),
            &precinct_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        assert!(csv.contains("PAPER,Precinct 1,contest-1,candidate_votes,cand-1,5"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_blank_votes,,1"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_valid_votes,,6"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_votes,,6"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,census,,100"));
    }

    #[test]
    fn ignores_group_zero_when_group_one_exists() {
        let xml = br#"
            <Owner name="EVS Electionware Enhanced XML Results File version 1.3">
                <JurisdictionMap>
                    <Jurisdiction id="1" title="Jurisdiction">
                        <Precinct id="precinct-1" name="Precinct 1">
                            <PrecinctReportingGroup reportingGroupId="0" ballotsCast="100" blanksCast="90"/>
                            <PrecinctReportingGroup reportingGroupId="1" ballotsCast="20" blanksCast="4"/>
                        </Precinct>
                        <Contest id="contest-source-1" altId1="contest-1" title="Contest">
                            <Candidate id="candidate-source-a" type="NORMAL" altId1="cand-1" name="Candidate A">
                                <CandidateReportingGroup reportingGroupId="0" totalVotes="99">
                                    <CandidateReportingGroupPrecinct refPrecinctId="precinct-1" votes="99"/>
                                </CandidateReportingGroup>
                                <CandidateReportingGroup reportingGroupId="1" totalVotes="7">
                                    <CandidateReportingGroupPrecinct refPrecinctId="precinct-1" votes="7"/>
                                </CandidateReportingGroup>
                            </Candidate>
                        </Contest>
                    </Jurisdiction>
                </JurisdictionMap>
            </Owner>
        "#;

        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &HashMap::new(),
            &precinct_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        assert!(csv.contains("PAPER,Precinct 1,contest-1,candidate_votes,cand-1,7"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_blank_votes,,4"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,census,,20"));
        assert!(!csv.contains("candidate_votes,cand-1,99"));
    }

    #[test]
    fn emits_precinct_rows_in_deterministic_order() {
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p2" name="Ward 2" />
                    <Precinct id="p1" name="Ward 1" />
                </JurisdictionMap>
                <Contest altId1="contest-1">
                    <ContestReportingGroup reportingGroupId="1">
                        <ContestReportingGroupVotes refPrecinctId="p2" ballotsCast="10" overVotes="0" underVotes="0" blankVotes="1" />
                        <ContestReportingGroupVotes refPrecinctId="p1" ballotsCast="20" overVotes="1" underVotes="2" blankVotes="2" />
                    </ContestReportingGroup>
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="17" />
                        <CandidatePrecinctVotes refPrecinctId="p2" votes="10" />
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        // Ward 1: candidate_sum(17) + over(1) + under(2) == ballots_cast(20) * max_votes(1).
        // Ward 2: candidate_sum(10) + over(0) + under(0) == ballots_cast(10) * max_votes(1).
        let expected = "\
channel,area_name,contest_external_id,field,candidate_external_id,value
PAPER,Ward 1,contest-1,total_votes,,20
PAPER,Ward 1,contest-1,total_valid_votes,,19
PAPER,Ward 1,contest-1,implicit_invalid,,1
PAPER,Ward 1,contest-1,explicit_invalid,,0
PAPER,Ward 1,contest-1,total_blank_votes,,2
PAPER,Ward 1,contest-1,census,,20
PAPER,Ward 1,contest-1,over_votes,,1
PAPER,Ward 1,contest-1,under_votes,,2
PAPER,Ward 1,contest-1,candidate_votes,cand-1,17
PAPER,Ward 2,contest-1,total_votes,,10
PAPER,Ward 2,contest-1,total_valid_votes,,10
PAPER,Ward 2,contest-1,implicit_invalid,,0
PAPER,Ward 2,contest-1,explicit_invalid,,0
PAPER,Ward 2,contest-1,total_blank_votes,,0
PAPER,Ward 2,contest-1,census,,10
PAPER,Ward 2,contest-1,over_votes,,0
PAPER,Ward 2,contest-1,under_votes,,0
PAPER,Ward 2,contest-1,candidate_votes,cand-1,10
";

        for _ in 0..5 {
            let (csv, errors) = convert_for_test(
                xml,
                VotingChannel::PAPER,
                &HashMap::new(),
                &precinct_area_names(),
            )
            .unwrap();
            assert!(errors.is_empty());
            assert_eq!(String::from_utf8(csv).unwrap(), expected);
        }
    }

    #[test]
    fn reports_a_validation_error_when_candidate_reporting_group_variant_has_no_group_one_data() {
        let xml = br#"
            <Owner>
                <JurisdictionMap>
                    <Jurisdiction id="1" title="Jurisdiction">
                        <Precinct id="precinct-1" name="Precinct 1">
                            <PrecinctReportingGroup reportingGroupId="0" ballotsCast="100" blanksCast="6"/>
                        </Precinct>
                        <Contest id="contest-source-1" altId1="contest-1" title="Contest">
                            <Candidate id="candidate-source-a" type="NORMAL" altId1="cand-1" name="Candidate A">
                                <CandidateReportingGroup reportingGroupId="0" totalVotes="58">
                                    <CandidateReportingGroupPrecinct refPrecinctId="precinct-1" votes="58"/>
                                </CandidateReportingGroup>
                            </Candidate>
                        </Contest>
                    </Jurisdiction>
                </JurisdictionMap>
            </Owner>
        "#;

        // A structural problem scoped to one contest is a validation error,
        // not a hard failure: the conversion still succeeds (with an empty
        // canonical CSV, since this file's only contest is the broken one).
        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &HashMap::new(),
            &precinct_area_names(),
        )
        .unwrap();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "xml_conversion_error");
        assert_eq!(errors[0].contest_external_id, Some("contest-1".to_string()));
        assert!(errors[0]
            .message
            .contains("Contest is missing CandidateReportingGroup data for reportingGroupId=1"));
        assert_eq!(
            String::from_utf8(csv).unwrap(),
            "channel,area_name,contest_external_id,field,candidate_external_id,value\n"
        );
    }

    #[test]
    fn a_broken_contest_does_not_block_other_contests_in_the_same_file() {
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1" />
                </JurisdictionMap>
                <Contest altId1="broken-contest">
                    <ContestReportingGroup reportingGroupId="1">
                        <ContestReportingGroupVotes refPrecinctId="p1" ballotsCast="20" overVotes="2" underVotes="5" blankVotes="4" />
                    </ContestReportingGroup>
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="7" />
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="7" />
                    </Candidate>
                </Contest>
                <Contest altId1="good-contest">
                    <ContestReportingGroup reportingGroupId="1">
                        <ContestReportingGroupVotes refPrecinctId="p1" ballotsCast="20" overVotes="2" underVotes="5" blankVotes="4" />
                    </ContestReportingGroup>
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="10" />
                    </Candidate>
                    <Candidate altId1="cand-2" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="3" />
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &HashMap::new(),
            &precinct_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].contest_external_id,
            Some("broken-contest".to_string())
        );
        assert!(errors[0]
            .message
            .contains("Duplicate CandidatePrecinctVotes"));

        assert!(!csv.contains("broken-contest"));
        assert!(csv.contains("PAPER,Precinct 1,good-contest,candidate_votes,cand-1,10"));
        assert!(csv.contains("PAPER,Precinct 1,good-contest,candidate_votes,cand-2,3"));
        assert!(csv.contains("PAPER,Precinct 1,good-contest,total_votes,,20"));
    }

    #[test]
    fn reports_a_validation_error_when_totals_do_not_reconcile() {
        // candidate_sum(5) + over(2) + under(5) = 12, but ballots_cast(20)
        // * max_votes(1, the default) = 20 — a genuine data-quality
        // problem in the source file, per the ES&S SOP's per-ballot
        // accounting identity.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1" />
                </JurisdictionMap>
                <Contest altId1="contest-1">
                    <ContestReportingGroup reportingGroupId="1">
                        <ContestReportingGroupVotes refPrecinctId="p1" ballotsCast="20" overVotes="2" underVotes="5" blankVotes="4" />
                    </ContestReportingGroup>
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="5" />
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &HashMap::new(),
            &precinct_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "ess_vote_reconciliation_mismatch");
        assert_eq!(errors[0].contest_external_id, Some("contest-1".to_string()));
        assert_eq!(errors[0].area_name, Some("Precinct 1".to_string()));
        // The rest of the contest still converts — this is a validation
        // warning about the source data, not a hard failure.
        assert!(csv.contains("PAPER,Precinct 1,contest-1,candidate_votes,cand-1,5"));
    }

    #[test]
    fn overvoted_ballot_on_a_vote_for_n_contest_counts_as_one_invalid_ballot() {
        // Reproduces a real ES&S test file: 3 ballots on a voteFor=4
        // contest — one voted correctly for cand-1..cand-4, one fully
        // overvoted, one left entirely blank. The overvoted ballot
        // contributes its whole max_votes allotment (4) to overVotes, and
        // the blank ballot contributes its whole allotment to underVotes.
        // implicit_invalid must come from overVotes / max_votes (1
        // overvoted *ballot*), not the raw overVotes slot count (4) —
        // otherwise total_valid_votes (3 ballots - 4 invalid) would
        // underflow to 0 instead of the correct 2. total_blank_votes must
        // similarly come from underVotes / max_votes (1 blank *ballot*),
        // not the raw underVotes slot count (4) — and must be at least the
        // precinct's whole-ballot blanksCast (1 here), since the one blank
        // ballot in this file is blank on the whole ballot too.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="3" blanksCast="1" />
                    </Precinct>
                </JurisdictionMap>
                <Contest altId1="contest-1">
                    <ContestReportingGroup reportingGroupId="1">
                        <ContestReportingGroupVotes refPrecinctId="p1" ballotsCast="3" overVotes="4" underVotes="4" blankVotes="8" />
                    </ContestReportingGroup>
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="1" />
                    </Candidate>
                    <Candidate altId1="cand-2" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="1" />
                    </Candidate>
                    <Candidate altId1="cand-3" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="1" />
                    </Candidate>
                    <Candidate altId1="cand-4" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="1" />
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let contest_vote_config = HashMap::from([(
            "contest-1".to_string(),
            ContestVoteConfig {
                min_votes: 0,
                max_votes: 4,
            },
        )]);
        let (csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &contest_vote_config,
            &precinct_area_names(),
        )
        .unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_votes,,3"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,implicit_invalid,,1"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_valid_votes,,2"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_blank_votes,,1"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,over_votes,,4"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,under_votes,,4"));
    }

    #[test]
    fn reports_a_validation_error_when_blank_votes_undercount_the_precincts_whole_ballot_blanks() {
        // The precinct's whole-ballot blanksCast (2) means at least 2
        // ballots are blank in every contest on the ballot, including this
        // one — but this contest's own underVotes (2) divided by max_votes
        // (2) only accounts for 1 blank ballot, an internally inconsistent
        // combination that can't reflect genuine source data.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1">
                        <PrecinctReportingGroup reportingGroupId="1" ballotsCast="10" blanksCast="2" />
                    </Precinct>
                </JurisdictionMap>
                <Contest altId1="contest-1">
                    <ContestReportingGroup reportingGroupId="1">
                        <ContestReportingGroupVotes refPrecinctId="p1" ballotsCast="10" overVotes="0" underVotes="2" blankVotes="2" />
                    </ContestReportingGroup>
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="18" />
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let contest_vote_config = HashMap::from([(
            "contest-1".to_string(),
            ContestVoteConfig {
                min_votes: 0,
                max_votes: 2,
            },
        )]);
        let (_csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &contest_vote_config,
            &precinct_area_names(),
        )
        .unwrap();

        // candidate_sum(18) + over(0) + under(2) == ballots_cast(10) * max_votes(2),
        // so the reconciliation and divisibility checks pass — only the
        // blanksCast lower-bound check fires.
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "ess_blank_votes_below_precinct_minimum");
        assert_eq!(errors[0].contest_external_id, Some("contest-1".to_string()));
        assert_eq!(errors[0].area_name, Some("Precinct 1".to_string()));
    }

    #[test]
    fn reports_a_validation_error_when_over_votes_is_not_an_exact_multiple_of_max_votes() {
        // candidate_sum(3) + over(5) + under(4) = 12 = ballots_cast(3) *
        // max_votes(4), so the reconciliation identity holds — but overVotes
        // (5) isn't itself an exact multiple of max_votes (4), so there's no
        // way to know how many ballots it represents. This is a distinct
        // data-quality problem from a reconciliation mismatch.
        let xml = br#"
            <ElectionReport>
                <JurisdictionMap>
                    <Precinct id="p1" name="Precinct 1" />
                </JurisdictionMap>
                <Contest altId1="contest-1">
                    <ContestReportingGroup reportingGroupId="1">
                        <ContestReportingGroupVotes refPrecinctId="p1" ballotsCast="3" overVotes="5" underVotes="4" blankVotes="9" />
                    </ContestReportingGroup>
                    <Candidate altId1="cand-1" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="3" />
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let contest_vote_config = HashMap::from([(
            "contest-1".to_string(),
            ContestVoteConfig {
                min_votes: 0,
                max_votes: 4,
            },
        )]);
        let (_csv, errors) = convert_for_test(
            xml,
            VotingChannel::PAPER,
            &contest_vote_config,
            &precinct_area_names(),
        )
        .unwrap();

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "ess_over_votes_not_divisible");
        assert_eq!(errors[0].contest_external_id, Some("contest-1".to_string()));
        assert_eq!(errors[0].area_name, Some("Precinct 1".to_string()));
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
            &HashMap::new(),
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
            &HashMap::new(),
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
        // under_votes are selection-slot counts recovered into ballot
        // counts via / max_votes, exactly like convert_precinct_grouped's
        // ContestReportingGroupVotes branch — see
        // reports_a_validation_error_when_totals_do_not_reconcile for the
        // precinct-grouped equivalent of the reconciliation identity this
        // relies on (candidate marks + over_votes + under_votes ==
        // total_votes * max_votes).
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

        let contest_vote_config = HashMap::from([(
            "contest-1".to_string(),
            ContestVoteConfig {
                min_votes: 1,
                max_votes: 1,
            },
        )]);
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

        let contest_vote_config = HashMap::from([(
            "council".to_string(),
            ContestVoteConfig {
                min_votes: 0,
                max_votes: 3,
            },
        )]);
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
            &HashMap::new(),
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
            &HashMap::new(),
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
