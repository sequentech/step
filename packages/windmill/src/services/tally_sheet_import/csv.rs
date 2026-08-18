// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::str::FromStr;

use csv::ReaderBuilder;
use sequent_core::types::tally_sheet_import::TallySheetImportValidationError;
use sequent_core::types::tally_sheets::{
    AreaContestResults, CandidateResults, InvalidVotes, VotingChannel,
};
use serde::Deserialize;
use tracing::instrument;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct BallotBoxImportKey {
    pub channel: VotingChannel,
    pub area_name: String,
    pub contest_external_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedBallotBoxImport {
    pub key: BallotBoxImportKey,
    pub content: AreaContestResults,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum CanonicalField {
    CandidateVotes,
    TotalBlankVotes,
    ImplicitInvalid,
    ExplicitInvalid,
    TotalValidVotes,
    TotalVotes,
    Census,
}

impl FromStr for CanonicalField {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "candidate_votes" => Ok(Self::CandidateVotes),
            "total_blank_votes" => Ok(Self::TotalBlankVotes),
            "implicit_invalid" => Ok(Self::ImplicitInvalid),
            "explicit_invalid" => Ok(Self::ExplicitInvalid),
            "total_valid_votes" => Ok(Self::TotalValidVotes),
            "total_votes" => Ok(Self::TotalVotes),
            "census" => Ok(Self::Census),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct CanonicalCsvRow {
    channel: String,
    area_name: String,
    contest_external_id: String,
    field: String,
    candidate_external_id: Option<String>,
    value: String,
}

#[derive(Debug, Default)]
struct BallotBoxAccumulator {
    total_votes: Option<u64>,
    total_valid_votes: Option<u64>,
    implicit_invalid: Option<u64>,
    explicit_invalid: Option<u64>,
    total_blank_votes: Option<u64>,
    census: Option<u64>,
    candidate_results: HashMap<String, CandidateResults>,
}

#[instrument(skip_all)]
pub fn parse_canonical_csv(
    bytes: &[u8],
) -> (
    Vec<ParsedBallotBoxImport>,
    Vec<TallySheetImportValidationError>,
) {
    let mut validation_errors = Vec::new();
    let mut reader = ReaderBuilder::new().has_headers(true).from_reader(bytes);
    let mut groups: HashMap<BallotBoxImportKey, BallotBoxAccumulator> = HashMap::new();

    for row_result in reader.deserialize::<CanonicalCsvRow>() {
        let row = match row_result {
            Ok(row) => row,
            Err(error) => {
                validation_errors.push(error_for_row(
                    "invalid_csv_row",
                    format!("Invalid canonical tally sheet CSV row: {error}"),
                    None,
                    None,
                    None,
                    None,
                    None,
                ));
                continue;
            }
        };

        let channel = match VotingChannel::from_str(row.channel.trim()) {
            Ok(channel) => channel,
            Err(_) => {
                validation_errors.push(error_for_row(
                    "invalid_channel",
                    format!("Invalid voting channel '{}'", row.channel),
                    None,
                    Some(row.area_name.clone()),
                    Some(row.contest_external_id.clone()),
                    row.candidate_external_id.clone(),
                    Some(row.field.clone()),
                ));
                continue;
            }
        };

        let field = match CanonicalField::from_str(row.field.trim()) {
            Ok(field) => field,
            Err(_) => {
                validation_errors.push(error_for_row(
                    "invalid_field",
                    format!("Invalid canonical tally sheet field '{}'", row.field),
                    Some(channel),
                    Some(row.area_name.clone()),
                    Some(row.contest_external_id.clone()),
                    row.candidate_external_id.clone(),
                    Some(row.field.clone()),
                ));
                continue;
            }
        };

        let value = match row.value.trim().parse::<u64>() {
            Ok(value) => value,
            Err(_) => {
                validation_errors.push(error_for_row(
                    "invalid_numeric_value",
                    format!("Invalid numeric value '{}'", row.value),
                    Some(channel),
                    Some(row.area_name.clone()),
                    Some(row.contest_external_id.clone()),
                    row.candidate_external_id.clone(),
                    Some(row.field.clone()),
                ));
                continue;
            }
        };

        if field == CanonicalField::CandidateVotes
            && row
                .candidate_external_id
                .as_ref()
                .map(|id| id.trim())
                .filter(|id| !id.is_empty())
                .is_none()
        {
            validation_errors.push(error_for_row(
                "missing_candidate_external_id",
                "candidate_external_id is required for candidate_votes rows".to_string(),
                Some(channel),
                Some(row.area_name.clone()),
                Some(row.contest_external_id.clone()),
                None,
                Some(row.field.clone()),
            ));
            continue;
        }

        let key = BallotBoxImportKey {
            channel,
            area_name: row.area_name.trim().to_string(),
            contest_external_id: row.contest_external_id.trim().to_string(),
        };
        let accumulator = groups.entry(key.clone()).or_default();
        apply_row(
            &mut validation_errors,
            &row,
            &key,
            field,
            value,
            accumulator,
        );
    }

    let mut imports = Vec::new();
    for (key, accumulator) in groups {
        validate_required_scalar_fields(&mut validation_errors, &key, &accumulator);
        let invalid_votes = InvalidVotes {
            total_invalid: Some(
                accumulator.implicit_invalid.unwrap_or(0)
                    + accumulator.explicit_invalid.unwrap_or(0),
            ),
            implicit_invalid: accumulator.implicit_invalid,
            explicit_invalid: accumulator.explicit_invalid,
        };
        imports.push(ParsedBallotBoxImport {
            key,
            content: AreaContestResults {
                area_id: String::new(),
                contest_id: String::new(),
                total_votes: accumulator.total_votes,
                total_valid_votes: accumulator.total_valid_votes,
                invalid_votes: Some(invalid_votes),
                total_blank_votes: accumulator.total_blank_votes,
                blank_ballots: None,
                census: accumulator.census,
                candidate_results: accumulator.candidate_results,
            },
        });
    }

    (imports, validation_errors)
}

fn apply_row(
    validation_errors: &mut Vec<TallySheetImportValidationError>,
    row: &CanonicalCsvRow,
    key: &BallotBoxImportKey,
    field: CanonicalField,
    value: u64,
    accumulator: &mut BallotBoxAccumulator,
) {
    match field {
        CanonicalField::CandidateVotes => {
            let candidate_external_id = row
                .candidate_external_id
                .as_ref()
                .map(|id| id.trim())
                .filter(|id| !id.is_empty());
            let Some(candidate_external_id) = candidate_external_id else {
                validation_errors.push(error_for_row(
                    "missing_candidate_external_id",
                    "candidate_external_id is required for candidate_votes rows".to_string(),
                    Some(key.channel.clone()),
                    Some(key.area_name.clone()),
                    Some(key.contest_external_id.clone()),
                    None,
                    Some(row.field.clone()),
                ));
                return;
            };

            let previous = accumulator.candidate_results.insert(
                candidate_external_id.to_string(),
                CandidateResults {
                    candidate_id: candidate_external_id.to_string(),
                    total_votes: Some(value),
                },
            );
            if previous.is_some() {
                validation_errors.push(error_for_row(
                    "duplicate_candidate_votes",
                    format!(
                        "Duplicate candidate_votes row for candidate '{candidate_external_id}'"
                    ),
                    Some(key.channel.clone()),
                    Some(key.area_name.clone()),
                    Some(key.contest_external_id.clone()),
                    Some(candidate_external_id.to_string()),
                    Some(row.field.clone()),
                ));
            }
        }
        CanonicalField::TotalBlankVotes => set_scalar(
            validation_errors,
            key,
            row,
            &mut accumulator.total_blank_votes,
            value,
        ),
        CanonicalField::ImplicitInvalid => set_scalar(
            validation_errors,
            key,
            row,
            &mut accumulator.implicit_invalid,
            value,
        ),
        CanonicalField::ExplicitInvalid => set_scalar(
            validation_errors,
            key,
            row,
            &mut accumulator.explicit_invalid,
            value,
        ),
        CanonicalField::TotalValidVotes => set_scalar(
            validation_errors,
            key,
            row,
            &mut accumulator.total_valid_votes,
            value,
        ),
        CanonicalField::TotalVotes => set_scalar(
            validation_errors,
            key,
            row,
            &mut accumulator.total_votes,
            value,
        ),
        CanonicalField::Census => {
            set_scalar(validation_errors, key, row, &mut accumulator.census, value)
        }
    }
}

fn set_scalar(
    validation_errors: &mut Vec<TallySheetImportValidationError>,
    key: &BallotBoxImportKey,
    row: &CanonicalCsvRow,
    target: &mut Option<u64>,
    value: u64,
) {
    if target.replace(value).is_some() {
        validation_errors.push(error_for_row(
            "duplicate_scalar_field",
            format!("Duplicate scalar row for field '{}'", row.field),
            Some(key.channel.clone()),
            Some(key.area_name.clone()),
            Some(key.contest_external_id.clone()),
            row.candidate_external_id.clone(),
            Some(row.field.clone()),
        ));
    }
}

fn validate_required_scalar_fields(
    validation_errors: &mut Vec<TallySheetImportValidationError>,
    key: &BallotBoxImportKey,
    accumulator: &BallotBoxAccumulator,
) {
    for (field, is_present) in [
        ("total_votes", accumulator.total_votes.is_some()),
        ("total_valid_votes", accumulator.total_valid_votes.is_some()),
        ("implicit_invalid", accumulator.implicit_invalid.is_some()),
        ("explicit_invalid", accumulator.explicit_invalid.is_some()),
        ("total_blank_votes", accumulator.total_blank_votes.is_some()),
        ("census", accumulator.census.is_some()),
    ] {
        if !is_present {
            validation_errors.push(error_for_row(
                "missing_scalar_field",
                format!("Missing required scalar row for field '{field}'"),
                Some(key.channel.clone()),
                Some(key.area_name.clone()),
                Some(key.contest_external_id.clone()),
                None,
                Some(field.to_string()),
            ));
        }
    }
}

fn error_for_row(
    code: &str,
    message: String,
    channel: Option<VotingChannel>,
    area_name: Option<String>,
    contest_external_id: Option<String>,
    candidate_external_id: Option<String>,
    field: Option<String>,
) -> TallySheetImportValidationError {
    TallySheetImportValidationError {
        code: code.to_string(),
        message,
        channel,
        area_name,
        contest_external_id,
        candidate_external_id,
        field,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_candidate_and_scalar_rows_into_ballot_box_import() {
        let csv = b"channel,area_name,contest_external_id,field,candidate_external_id,value\
\nPAPER,Precinct 1,contest-1,candidate_votes,cand-1,7\
\nPAPER,Precinct 1,contest-1,candidate_votes,cand-2,3\
\nPAPER,Precinct 1,contest-1,total_blank_votes,,2\
\nPAPER,Precinct 1,contest-1,implicit_invalid,,1\
\nPAPER,Precinct 1,contest-1,explicit_invalid,,4\
\nPAPER,Precinct 1,contest-1,total_valid_votes,,12\
\nPAPER,Precinct 1,contest-1,total_votes,,17\
\nPAPER,Precinct 1,contest-1,census,,20\n";

        let (imports, errors) = parse_canonical_csv(csv);

        assert!(errors.is_empty());
        assert_eq!(imports.len(), 1);
        let import = &imports[0];
        assert_eq!(import.key.channel, VotingChannel::PAPER);
        assert_eq!(import.key.area_name, "Precinct 1");
        assert_eq!(import.key.contest_external_id, "contest-1");
        assert_eq!(import.content.total_blank_votes, Some(2));
        assert_eq!(import.content.total_valid_votes, Some(12));
        assert_eq!(import.content.total_votes, Some(17));
        assert_eq!(import.content.census, Some(20));
        assert_eq!(
            import
                .content
                .invalid_votes
                .as_ref()
                .and_then(|votes| votes.total_invalid),
            Some(5)
        );
        assert_eq!(
            import
                .content
                .candidate_results
                .get("cand-1")
                .and_then(|candidate| candidate.total_votes),
            Some(7)
        );
    }

    #[test]
    fn reports_missing_candidate_external_id_for_candidate_votes() {
        let csv = b"channel,area_name,contest_external_id,field,candidate_external_id,value\
\nPAPER,Precinct 1,contest-1,candidate_votes,,7\n";

        let (_imports, errors) = parse_canonical_csv(csv);

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "missing_candidate_external_id");
    }

    #[test]
    fn reports_missing_required_scalar_fields() {
        let csv = b"channel,area_name,contest_external_id,field,candidate_external_id,value\
\nPAPER,Precinct 1,contest-1,candidate_votes,cand-1,0\
\nPAPER,Precinct 1,contest-1,total_votes,,0\n";

        let (_imports, errors) = parse_canonical_csv(csv);
        let missing_fields = errors
            .into_iter()
            .filter(|error| error.code == "missing_scalar_field")
            .map(|error| error.field)
            .collect::<Vec<_>>();

        assert_eq!(
            missing_fields,
            vec![
                Some("total_valid_votes".to_string()),
                Some("implicit_invalid".to_string()),
                Some("explicit_invalid".to_string()),
                Some("total_blank_votes".to_string()),
                Some("census".to_string()),
            ]
        );
    }
}
