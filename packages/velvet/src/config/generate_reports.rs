// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::{
    ballot::ConsolidatedReportPolicy,
    types::{
        ceremonies::TallyType,
        date_time::{DateFormat, TimeZone},
        hasura::core::TallySessionConfiguration,
        number_format::{deserialize_lenient_number_format_policy, NumberFormatPolicy},
        templates::PrintToPdfOptionsLocal,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, str::FromStr};
use strum_macros::EnumString;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PipeConfigGenerateReports {
    pub enable_pdfs: bool,
    pub report_content_template: Option<String>,
    pub pdf_options: Option<PrintToPdfOptionsLocal>,
    pub execution_annotations: HashMap<String, String>,
    pub system_template: String,
    pub extra_data: Value,
    pub tally_type: TallyType,
    pub tally_session_configuration: Option<TallySessionConfiguration>,
    /// The election event's number format, which the reports write their
    /// figures in. Configs written before it existed lack it, and configs
    /// written by a newer version may name one this version doesn't know:
    /// both use the default.
    #[serde(default, deserialize_with = "deserialize_lenient_number_format_policy")]
    pub number_format_policy: Option<NumberFormatPolicy>,
}

#[derive(Serialize, Deserialize, Debug, Default, EnumString)]
pub enum CandidatesOrderPolicy {
    #[default]
    SortByWinningPosition,
    AsInBallot,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ContestReportConfig {
    pub candidates_order: CandidatesOrderPolicy,
}

pub const CONTEST_REPORT_CONFIG: &'static str = "sequent:velvet:contest-report-config";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_config_written_before_number_format_policies_still_parses() {
        let config: PipeConfigGenerateReports = serde_json::from_value(json!({
            "enable_pdfs": false,
            "report_content_template": null,
            "pdf_options": null,
            "execution_annotations": {},
            "system_template": "",
            "extra_data": {},
            "tally_type": "ELECTORAL_RESULTS",
            "tally_session_configuration": null
        }))
        .unwrap();

        assert_eq!(config.number_format_policy, None);
    }

    #[test]
    fn a_config_with_a_number_format_this_version_does_not_know_uses_the_default() {
        for policy in [json!("no-such-format"), json!(7), Value::Null] {
            let config: PipeConfigGenerateReports = serde_json::from_value(json!({
                "enable_pdfs": false,
                "report_content_template": null,
                "pdf_options": null,
                "execution_annotations": {},
                "system_template": "",
                "extra_data": {},
                "tally_type": "ELECTORAL_RESULTS",
                "tally_session_configuration": null,
                "number_format_policy": policy
            }))
            .unwrap();

            assert_eq!(config.number_format_policy, None);
        }
    }

    #[test]
    fn the_number_format_policy_is_written_by_its_code() {
        let config = serde_json::to_value(PipeConfigGenerateReports {
            number_format_policy: Some(NumberFormatPolicy::PeriodComma),
            ..Default::default()
        })
        .unwrap();

        assert_eq!(config["number_format_policy"], json!("period-comma"));
        assert_eq!(
            serde_json::from_value::<PipeConfigGenerateReports>(config)
                .unwrap()
                .number_format_policy,
            Some(NumberFormatPolicy::PeriodComma)
        );
    }
}
