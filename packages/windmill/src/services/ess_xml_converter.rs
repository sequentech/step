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

#[derive(Debug, Clone, Default)]
struct ContestPrecinctTotals {
    ballots_cast: u64,
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
    }
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
) -> Result<(Vec<u8>, Vec<TallySheetImportValidationError>)> {
    convert_ess_enhanced_xml_to_csv_for_reporting_group(
        xml_bytes,
        selected_channel,
        DEFAULT_IMPORT_REPORTING_GROUP_ID,
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
#[instrument(skip_all, err)]
pub fn convert_ess_enhanced_xml_to_csv_for_reporting_group(
    xml_bytes: &[u8],
    selected_channel: VotingChannel,
    reporting_group_id: &str,
) -> Result<(Vec<u8>, Vec<TallySheetImportValidationError>)> {
    let xml = std::str::from_utf8(xml_bytes).context("ES&S XML import must be valid UTF-8")?;
    let document = Document::parse(xml).context("Invalid ES&S Enhanced XML")?;
    let precinct_names = precinct_names_by_id(&document)?;
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
            let total_blank_votes = totals.blank_votes;
            let implicit_invalid =
                totals.over_votes + totals.under_votes.saturating_sub(total_blank_votes);
            let explicit_invalid = 0;
            let total_valid_votes = candidate_votes_sum + total_blank_votes;
            let total_votes = total_valid_votes + implicit_invalid + explicit_invalid;

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
                totals.ballots_cast,
            )?;

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
            entry.ballots_cast +=
                parse_u64_attr(votes, "ballotsCast", "ContestReportingGroupVotes")?;
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
            entry.ballots_cast +=
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
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="7" />
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

        let (csv, errors) = convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER).unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(errors.is_empty());
        assert!(csv.contains("PAPER,Precinct 1,contest-1,implicit_invalid,,3"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_blank_votes,,4"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_valid_votes,,14"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_votes,,17"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,candidate_votes,cand-1,7"));
        assert!(!csv.contains("ignored-overvotes"));
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

        let (csv, errors) = convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER).unwrap();
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

        let (csv, errors) = convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER).unwrap();
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
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="7" />
                        <CandidatePrecinctVotes refPrecinctId="p2" votes="4" />
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let expected = "\
channel,area_name,contest_external_id,field,candidate_external_id,value
PAPER,Ward 1,contest-1,total_votes,,10
PAPER,Ward 1,contest-1,total_valid_votes,,9
PAPER,Ward 1,contest-1,implicit_invalid,,1
PAPER,Ward 1,contest-1,explicit_invalid,,0
PAPER,Ward 1,contest-1,total_blank_votes,,2
PAPER,Ward 1,contest-1,census,,20
PAPER,Ward 1,contest-1,candidate_votes,cand-1,7
PAPER,Ward 2,contest-1,total_votes,,5
PAPER,Ward 2,contest-1,total_valid_votes,,5
PAPER,Ward 2,contest-1,implicit_invalid,,0
PAPER,Ward 2,contest-1,explicit_invalid,,0
PAPER,Ward 2,contest-1,total_blank_votes,,1
PAPER,Ward 2,contest-1,census,,10
PAPER,Ward 2,contest-1,candidate_votes,cand-1,4
";

        for _ in 0..5 {
            let (csv, errors) = convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER).unwrap();
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
        let (csv, errors) = convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER).unwrap();
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
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="7" />
                    </Candidate>
                    <Candidate altId1="cand-2" type="NORMAL">
                        <CandidatePrecinctVotes refPrecinctId="p1" votes="3" />
                    </Candidate>
                </Contest>
            </ElectionReport>
        "#;

        let (csv, errors) = convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER).unwrap();
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
        assert!(csv.contains("PAPER,Precinct 1,good-contest,candidate_votes,cand-1,7"));
        assert!(csv.contains("PAPER,Precinct 1,good-contest,candidate_votes,cand-2,3"));
        assert!(csv.contains("PAPER,Precinct 1,good-contest,total_votes,,17"));
    }
}
