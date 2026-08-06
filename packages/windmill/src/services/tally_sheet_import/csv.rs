// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use csv::ReaderBuilder;
use sequent_core::types::tally_sheet_import::TallySheetImportValidationError;
use sequent_core::types::tally_sheets::{
    AreaContestResults, CandidateResults, InvalidVotes, VotingChannel,
};
use serde::Deserialize;
use serde_json::Value;
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

#[derive(Debug, Clone, Eq, PartialEq)]
enum CanonicalField {
    CandidateVotes,
    TotalBlankVotes,
    ImplicitInvalid,
    ExplicitInvalid,
    TotalValidVotes,
    TotalVotes,
    Census,
    /// A field the source format's converter declared it emits (see
    /// `allowed_annotation_fields`). Carried through as an unvalidated
    /// `annotations` entry (no duplicate-row or required-field checks) so
    /// source-specific extra data — e.g. ES&S's raw slot counts — flows
    /// through without becoming a canonical scalar.
    Annotation(String),
}

impl CanonicalField {
    /// Parses a `field` column value. `allowed_annotation_fields` is the
    /// set declared by whichever converter produced this file; anything
    /// outside both it and the canonical set above is rejected.
    ///
    /// Deliberately not a `FromStr` impl: which annotations are valid
    /// depends on where the file came from, so it can't be decided from the
    /// string alone. Accepting any unrecognised name as an annotation (the
    /// obvious alternative) means a mistyped canonical field like
    /// `total_vots` parses happily, silently dropping the scalar it was
    /// meant to set — the ballot box then fails later with a confusing
    /// "missing required field", or worse, passes carrying a stale value.
    fn parse(value: &str, allowed_annotation_fields: &HashSet<String>) -> Result<Self, ()> {
        match value {
            "candidate_votes" => Ok(Self::CandidateVotes),
            "total_blank_votes" => Ok(Self::TotalBlankVotes),
            "implicit_invalid" => Ok(Self::ImplicitInvalid),
            "explicit_invalid" => Ok(Self::ExplicitInvalid),
            "total_valid_votes" => Ok(Self::TotalValidVotes),
            "total_votes" => Ok(Self::TotalVotes),
            "census" => Ok(Self::Census),
            other if allowed_annotation_fields.contains(other) => {
                Ok(Self::Annotation(other.to_string()))
            }
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
    /// Values from any field row outside the fixed scalar set, keyed by
    /// field name. Last value wins for a repeated key — unlike the fixed
    /// scalar fields, duplicates aren't treated as an error here.
    annotations: HashMap<String, u64>,
}

/// `allowed_annotation_fields` is the set of extra, non-canonical `field`
/// values the source format's converter declared it emits — see
/// `super::annotations::allowed_annotation_fields`. Anything outside it and
/// the canonical set is reported as `invalid_field`.
#[instrument(skip_all)]
pub fn parse_canonical_csv(
    bytes: &[u8],
    allowed_annotation_fields: &HashSet<String>,
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

        let field = match CanonicalField::parse(row.field.trim(), allowed_annotation_fields) {
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
        let annotations = if accumulator.annotations.is_empty() {
            None
        } else {
            Some(Value::Object(
                accumulator
                    .annotations
                    .into_iter()
                    .map(|(name, value)| (name, Value::from(value)))
                    .collect(),
            ))
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
                census: accumulator.census,
                candidate_results: accumulator.candidate_results,
                annotations,
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
        CanonicalField::Annotation(name) => {
            accumulator.annotations.insert(name, value);
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
        params: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ess_xml_converter::EssAnnotationField;

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

        let (imports, errors) = parse_canonical_csv(csv, &HashSet::new());

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

        let (_imports, errors) = parse_canonical_csv(csv, &HashSet::new());

        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "missing_candidate_external_id");
    }

    #[test]
    fn reports_missing_required_scalar_fields() {
        let csv = b"channel,area_name,contest_external_id,field,candidate_external_id,value\
\nPAPER,Precinct 1,contest-1,candidate_votes,cand-1,0\
\nPAPER,Precinct 1,contest-1,total_votes,,0\n";

        let (_imports, errors) = parse_canonical_csv(csv, &HashSet::new());
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

    #[test]
    fn carries_known_annotation_fields_through_unvalidated() {
        let csv = b"channel,area_name,contest_external_id,field,candidate_external_id,value\
\nPAPER,Area A,contest-1,candidate_votes,cand-1,4\
\nPAPER,Area A,contest-1,total_blank_votes,,1\
\nPAPER,Area A,contest-1,implicit_invalid,,1\
\nPAPER,Area A,contest-1,explicit_invalid,,0\
\nPAPER,Area A,contest-1,total_valid_votes,,2\
\nPAPER,Area A,contest-1,total_votes,,3\
\nPAPER,Area A,contest-1,census,,3\
\nPAPER,Area A,contest-1,over_votes,,4\
\nPAPER,Area A,contest-1,under_votes,,4\n";

        let (imports, errors) = parse_canonical_csv(csv, &EssAnnotationField::all_names());

        assert!(errors.is_empty());
        let annotations = imports[0].content.annotations.as_ref().unwrap();
        assert_eq!(annotations["over_votes"], 4);
        assert_eq!(annotations["under_votes"], 4);
    }

    #[test]
    fn reports_a_typod_field_name_instead_of_absorbing_it_as_an_annotation() {
        // `total_vots` used to parse as an annotation, so the row it was
        // meant to set was silently dropped and the ballot box failed later
        // with a confusing "missing required field" instead. Checked with
        // the ES&S allowlist active, so it's the name not being *declared*
        // that rejects it, not the absence of any annotations at all.
        let csv = b"channel,area_name,contest_external_id,field,candidate_external_id,value\
\nPAPER,Area A,contest-1,total_vots,,17\n";

        let (_imports, errors) = parse_canonical_csv(csv, &EssAnnotationField::all_names());

        let invalid_field_errors = errors
            .iter()
            .filter(|error| error.code == "invalid_field")
            .collect::<Vec<_>>();
        assert_eq!(invalid_field_errors.len(), 1);
        assert_eq!(
            invalid_field_errors[0].field,
            Some("total_vots".to_string())
        );
    }

    #[test]
    fn rejects_another_formats_annotation_fields_for_a_canonical_csv_source() {
        // A canonical CSV source declares no annotation fields, so ES&S's
        // extra columns aren't silently accepted just because some other
        // source format happens to emit them.
        let csv = b"channel,area_name,contest_external_id,field,candidate_external_id,value\
\nPAPER,Area A,contest-1,over_votes,,4\n";

        let (_imports, errors) = parse_canonical_csv(csv, &HashSet::new());

        assert_eq!(
            errors
                .iter()
                .filter(|error| error.code == "invalid_field")
                .count(),
            1
        );
    }
}
