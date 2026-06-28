// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use csv::Writer;
use roxmltree::{Document, Node};
use sequent_core::types::tally_sheets::VotingChannel;

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

pub fn convert_ess_enhanced_xml_to_csv(
    xml_bytes: &[u8],
    selected_channel: VotingChannel,
) -> Result<Vec<u8>> {
    let xml = std::str::from_utf8(xml_bytes).context("ES&S XML import must be valid UTF-8")?;
    let document = Document::parse(xml).context("Invalid ES&S Enhanced XML")?;
    let precinct_names = precinct_names_by_id(&document)?;
    let mut writer = Writer::from_writer(Vec::new());

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
        let contest_external_id = required_attr(contest, "altId1", "Contest")?;
        if contest_external_id.trim().is_empty() {
            return Err(anyhow!("Contest is missing altId1 import id"));
        }
        let totals_by_precinct = contest_totals_by_precinct(contest)?;
        let candidates = normal_candidate_votes(contest)?;

        for (precinct_id, totals) in totals_by_precinct {
            let Some(area_name) = precinct_names.get(&precinct_id) else {
                return Err(anyhow!(
                    "Contest references precinct id '{}' not present in JurisdictionMap",
                    precinct_id
                ));
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

    writer.into_inner().map_err(|err| anyhow!(err))
}

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

fn contest_totals_by_precinct(
    contest: Node<'_, '_>,
) -> Result<HashMap<String, ContestPrecinctTotals>> {
    let mut totals_by_precinct: HashMap<String, ContestPrecinctTotals> = HashMap::new();
    for group in contest
        .children()
        .filter(|node| node.has_tag_name("ContestReportingGroup"))
    {
        let reporting_group_id = required_attr(group, "reportingGroupId", "ContestReportingGroup")?;
        if reporting_group_id == "0" {
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

        let csv = convert_ess_enhanced_xml_to_csv(xml, VotingChannel::PAPER).unwrap();
        let csv = String::from_utf8(csv).unwrap();

        assert!(csv.contains("PAPER,Precinct 1,contest-1,implicit_invalid,,3"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_blank_votes,,4"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_valid_votes,,14"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,total_votes,,17"));
        assert!(csv.contains("PAPER,Precinct 1,contest-1,candidate_votes,cand-1,7"));
        assert!(!csv.contains("ignored-overvotes"));
    }
}
