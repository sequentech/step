// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The results CSV of a transmission package: the EML's votes, one row per
//! candidate and then each contest's counts, for canvassing systems that
//! import CSV rather than XML.

use super::eml_generator::{MiruAreaAnnotations, MiruElectionAnnotations};
use super::eml_types::{EMLContest, EMLFile};
use anyhow::{Context, Result};
use strum_macros::{Display, EnumString};

/// The name the results CSV of a transaction is stored with.
pub fn csv_name(transaction_id: &str) -> String {
    format!("er_{transaction_id}.csv")
}

pub const RESULTS_CSV_HEADER: [&str; 11] = [
    "transaction_id",
    "station_id",
    "post",
    "country",
    "contest_id",
    "contest",
    "row",
    "candidate_id",
    "candidate",
    "party_id",
    "votes",
];

/// What a row of the CSV holds: a candidate's votes or one of the contest's
/// counts, named by the EML's `CountMetric` id.
#[derive(Display, EnumString, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultsCsvRow {
    #[strum(serialize = "CANDIDATE")]
    Candidate,
    #[strum(serialize = "RV")]
    RegisteredVoters,
    #[strum(serialize = "BB")]
    BallotsInBox,
    #[strum(serialize = "VB")]
    ValidBallots,
    #[strum(serialize = "IB")]
    InvalidBallots,
    #[strum(serialize = "VV")]
    Votes,
    #[strum(serialize = "UV")]
    UnderVotes,
}

/// The counts each contest lists after its candidates, in this order.
pub const CONTEST_COUNTS: [ResultsCsvRow; 6] = [
    ResultsCsvRow::RegisteredVoters,
    ResultsCsvRow::BallotsInBox,
    ResultsCsvRow::ValidBallots,
    ResultsCsvRow::InvalidBallots,
    ResultsCsvRow::Votes,
    ResultsCsvRow::UnderVotes,
];

/// Where the returns are from, as each row repeats it.
struct Place<'a> {
    transaction_id: &'a str,
    station_id: &'a str,
    post: &'a str,
    country: &'a str,
}

fn contest_rows(place: &Place<'_>, contest: &EMLContest) -> Vec<[String; 11]> {
    let row =
        |kind: ResultsCsvRow, candidate_id: &str, candidate: &str, party_id: &str, votes: i64| {
            [
                place.transaction_id.to_string(),
                place.station_id.to_string(),
                place.post.to_string(),
                place.country.to_string(),
                contest.identifier.id_number.clone(),
                contest.identifier.name.clone(),
                kind.to_string(),
                candidate_id.to_string(),
                candidate.to_string(),
                party_id.to_string(),
                votes.to_string(),
            ]
        };
    let mut rows = vec![];
    for selection in &contest.total_votes.selections {
        for candidate in &selection.candidates {
            rows.push(row(
                ResultsCsvRow::Candidate,
                &candidate.identifier.id_number,
                &candidate.identifier.name,
                candidate.affiliation.identifier.id_number.trim(),
                selection.valid_votes,
            ));
        }
    }
    for count in CONTEST_COUNTS {
        let id = count.to_string();
        if let Some(metric) = contest
            .total_votes
            .count_metrics
            .iter()
            .find(|metric| metric.id == id)
        {
            rows.push(row(count, "", &metric.kind, "", metric.datum));
        }
    }
    rows
}

/// The CSV of the returns `eml`, with the same votes for every candidate.
pub fn results_csv(
    eml: &EMLFile,
    election_annotations: &MiruElectionAnnotations,
    area_annotations: &MiruAreaAnnotations,
) -> Result<String> {
    let place = Place {
        transaction_id: &eml.header.transaction_id,
        station_id: &area_annotations.station_id,
        post: &election_annotations.election_name,
        country: &area_annotations.country,
    };
    let mut writer = csv::Writer::from_writer(vec![]);
    writer
        .write_record(RESULTS_CSV_HEADER)
        .context("Error writing the results CSV's header")?;
    for count in &eml.counts {
        for election in &count.elections {
            for contest in &election.contests {
                for row in contest_rows(&place, contest) {
                    writer
                        .write_record(&row)
                        .context("Error writing a row of the results CSV")?;
                }
            }
        }
    }
    let bytes = writer
        .into_inner()
        .context("Error finishing the results CSV")?;
    String::from_utf8(bytes).context("The results CSV is not UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::consolidation::eml_types::*;
    use std::str::FromStr;

    fn identifier(id: &str, name: &str) -> EMLIdentifier {
        EMLIdentifier {
            id_number: id.to_string(),
            name: name.to_string(),
        }
    }

    fn selection(id: &str, name: &str, party: &str, votes: i64) -> EMLSelection {
        EMLSelection {
            candidates: vec![EMLCandidate {
                identifier: identifier(id, name),
                status_details: vec![],
                affiliation: EMLAffiliation {
                    identifier: identifier(party, "Party"),
                    party: "Party".to_string(),
                },
            }],
            valid_votes: votes,
        }
    }

    fn metric(id: &str, kind: &str, datum: i64) -> EMLCountMetric {
        EMLCountMetric {
            kind: kind.to_string(),
            id: id.to_string(),
            datum,
        }
    }

    fn eml() -> EMLFile {
        EMLFile {
            id: "tally".to_string(),
            header: EMLHeader {
                transaction_id: "0000001234".to_string(),
                issue_date: "2028-05-08T19:00:00".to_string(),
                official_status_detail: EMLOfficialStatusDetail {
                    official_status: "official".to_string(),
                    status_date: "2028-05-08".to_string(),
                },
            },
            counts: vec![EMLCount {
                identifier: identifier("2028", "OV 2028"),
                elections: vec![EMLElection {
                    identifier: identifier("9001", "Madrid PE"),
                    contests: vec![EMLContest {
                        identifier: identifier("1", "PRESIDENT"),
                        total_votes: EMLTotalVotes {
                            count_metrics: vec![
                                metric("OV", "Total Number of Over Votes", 0),
                                metric("UV", "Total Number of Under Votes", 1),
                                metric("VV", "Total Number of Votes Actually", 9),
                                metric("RV", "Total Number of Registered Voters", 50),
                                metric("VB", "Total Number of Valid Ballots", 10),
                                metric("BB", "Total Number of Ballots In Ballot Box", 11),
                                metric("IB", "Total Number of Invalid Ballots", 1),
                            ],
                            selections: vec![
                                selection("11", "DELA CRUZ, Juan", "P1", 6),
                                selection("12", "SANTOS, \"Maria\"", " ", 3),
                            ],
                        },
                    }],
                }],
            }],
        }
    }

    fn election() -> MiruElectionAnnotations {
        MiruElectionAnnotations {
            election_id: "9001".to_string(),
            election_name: "Madrid PE".to_string(),
            geographical_area: "Europe".to_string(),
            post: "Madrid".to_string(),
            precinct_code: "P-9001".to_string(),
            pollcenter_code: "".to_string(),
        }
    }

    fn area() -> MiruAreaAnnotations {
        MiruAreaAnnotations {
            ccs_servers: vec![],
            station_id: "90010001".to_string(),
            station_name: "Spain".to_string(),
            threshold: 2,
            sbei_ids: vec![],
            country: "Spain".to_string(),
            registered_voters: 50,
        }
    }

    #[test]
    fn each_candidate_has_a_row_with_the_emls_votes_and_each_contest_its_counts() {
        let csv = results_csv(&eml(), &election(), &area()).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(
            lines,
            vec![
                "transaction_id,station_id,post,country,contest_id,contest,row,candidate_id,candidate,party_id,votes",
                "0000001234,90010001,Madrid PE,Spain,1,PRESIDENT,CANDIDATE,11,\"DELA CRUZ, Juan\",P1,6",
                "0000001234,90010001,Madrid PE,Spain,1,PRESIDENT,CANDIDATE,12,\"SANTOS, \"\"Maria\"\"\",,3",
                "0000001234,90010001,Madrid PE,Spain,1,PRESIDENT,RV,,Total Number of Registered Voters,,50",
                "0000001234,90010001,Madrid PE,Spain,1,PRESIDENT,BB,,Total Number of Ballots In Ballot Box,,11",
                "0000001234,90010001,Madrid PE,Spain,1,PRESIDENT,VB,,Total Number of Valid Ballots,,10",
                "0000001234,90010001,Madrid PE,Spain,1,PRESIDENT,IB,,Total Number of Invalid Ballots,,1",
                "0000001234,90010001,Madrid PE,Spain,1,PRESIDENT,VV,,Total Number of Votes Actually,,9",
                "0000001234,90010001,Madrid PE,Spain,1,PRESIDENT,UV,,Total Number of Under Votes,,1",
            ]
        );
    }

    #[test]
    fn the_csv_reads_back_with_the_same_votes_as_the_eml() {
        let eml = eml();
        let csv = results_csv(&eml, &election(), &area()).unwrap();
        let mut reader = csv::Reader::from_reader(csv.as_bytes());
        let votes: Vec<(String, i64)> = reader
            .records()
            .map(|record| record.unwrap())
            .filter(|record| ResultsCsvRow::from_str(&record[6]) == Ok(ResultsCsvRow::Candidate))
            .map(|record| (record[7].to_string(), record[10].parse().unwrap()))
            .collect();
        let expected: Vec<(String, i64)> = eml.counts[0].elections[0].contests[0]
            .total_votes
            .selections
            .iter()
            .map(|selection| {
                (
                    selection.candidates[0].identifier.id_number.clone(),
                    selection.valid_votes,
                )
            })
            .collect();
        assert_eq!(votes, expected);
    }

    #[test]
    fn a_contest_without_a_count_leaves_it_out_and_returns_without_contests_have_only_the_header() {
        let mut missing = eml();
        missing.counts[0].elections[0].contests[0]
            .total_votes
            .count_metrics
            .retain(|metric| metric.id != "UV");
        let csv = results_csv(&missing, &election(), &area()).unwrap();
        assert!(!csv.contains(",UV,"));
        assert_eq!(csv.lines().count(), 8);

        let mut empty = eml();
        empty.counts[0].elections[0].contests.clear();
        let csv = results_csv(&empty, &election(), &area()).unwrap();
        assert_eq!(csv.lines().count(), 1);
    }

    #[test]
    fn rows_are_named_by_the_emls_metric_ids() {
        assert_eq!(csv_name("0000001234"), "er_0000001234.csv");
        assert_eq!(ResultsCsvRow::RegisteredVoters.to_string(), "RV");
        assert_eq!(
            ResultsCsvRow::from_str("CANDIDATE"),
            Ok(ResultsCsvRow::Candidate)
        );
        assert!(ResultsCsvRow::from_str("OV").is_err());
    }
}
