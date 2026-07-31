// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::{BTreeMap, HashMap};

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

#[derive(Debug, Clone)]
struct CandidateVotes {
    external_id: String,
    votes_by_precinct: HashMap<String, u64>,
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

#[instrument(skip_all, err)]
pub fn convert_ess_enhanced_xml_to_csv(
    xml_bytes: &[u8],
    selected_channel: VotingChannel,
    contest_vote_config: &HashMap<String, ContestVoteConfig>,
) -> Result<(Vec<u8>, Vec<TallySheetImportValidationError>)> {
    convert_ess_enhanced_xml_to_csv_for_reporting_group(
        xml_bytes,
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
#[instrument(skip_all, err)]
pub fn convert_ess_enhanced_xml_to_csv_for_reporting_group(
    xml_bytes: &[u8],
    selected_channel: VotingChannel,
    reporting_group_id: &str,
    contest_vote_config: &HashMap<String, ContestVoteConfig>,
) -> Result<(Vec<u8>, Vec<TallySheetImportValidationError>)> {
    let xml = std::str::from_utf8(xml_bytes).context("ES&S XML import must be valid UTF-8")?;
    let document = Document::parse(xml).context("Invalid ES&S Enhanced XML")?;
    let precinct_names = precinct_names_by_id(&document)?;
    let precinct_blanks_cast = precinct_blanks_cast_by_id(&document, reporting_group_id);
    let mut writer = Writer::from_writer(Vec::new());
    let mut validation_errors = Vec::new();

    writer.write_record([
        "channel",
        "area_name",
        "contest_external_id",
        "field",
        "candidate_external_id",
        "value",
    ])?;

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
            match resolve_contest_data(contest, &document, reporting_group_id) {
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
                        .votes_by_precinct
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
                if let Some(&precinct_blanks_cast_value) = precinct_blanks_cast.get(&precinct_id)
                {
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
                        .votes_by_precinct
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
        let mut votes_by_precinct = HashMap::new();
        for votes in candidate
            .children()
            .filter(|node| node.has_tag_name("CandidatePrecinctVotes"))
        {
            let precinct_id = required_attr(votes, "refPrecinctId", "CandidatePrecinctVotes")?;
            let vote_count = parse_u64_attr(votes, "votes", "CandidatePrecinctVotes")?;
            if votes_by_precinct
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
            votes_by_precinct,
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
            votes_by_precinct,
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

        let (csv, errors) =
            convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER, &HashMap::new()).unwrap();
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
        let (csv, errors) =
            convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER, &contest_vote_config)
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
        let (csv, errors) =
            convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER, &contest_vote_config)
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

        let (csv, errors) =
            convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER, &HashMap::new()).unwrap();
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

        let (csv, errors) =
            convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER, &HashMap::new()).unwrap();
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

        let (csv, errors) =
            convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER, &HashMap::new()).unwrap();
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
            let (csv, errors) =
                convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER, &HashMap::new())
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
        let (csv, errors) =
            convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER, &HashMap::new()).unwrap();
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

        let (csv, errors) =
            convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER, &HashMap::new()).unwrap();
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

        let (csv, errors) =
            convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER, &HashMap::new()).unwrap();
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
        let (csv, errors) =
            convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER, &contest_vote_config)
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
        let (_csv, errors) =
            convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER, &contest_vote_config)
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
        let (_csv, errors) =
            convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER, &contest_vote_config)
                .unwrap();

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "ess_over_votes_not_divisible");
        assert_eq!(errors[0].contest_external_id, Some("contest-1".to_string()));
        assert_eq!(errors[0].area_name, Some("Precinct 1".to_string()));
    }
}
