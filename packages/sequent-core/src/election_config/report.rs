// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Report definitions, as they appear in an import bundle.
//!
//! Moved here from `windmill::postgres::reports` and
//! `windmill::services::reports::template_renderer` so that the tools which
//! *write* an import describe reports the same way the importer reads them.
//! windmill re-exports these, so its own call sites are unchanged.
//!
//! The database mapping (`ReportWrapper`, `TryFrom<Row>`) deliberately stays in
//! windmill: it needs `tokio_postgres`, which has no place in a module that has
//! to compile to WASM.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum_macros::{Display, EnumString, IntoStaticStr};

/// How a generated report document is protected.
///
/// Serialized `snake_case`. There are exactly two: a report is either readable
/// or encrypted with a password configured alongside it.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    IntoStaticStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum EReportEncryption {
    Unencrypted,
    ConfiguredPassword,
}

/// Schedule for a report that regenerates itself and mails the result.
///
/// Every field defaults, because a report without a cron config is the normal
/// case and an absent key must not fail deserialization.
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone, Default)]
pub struct ReportCronConfig {
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub last_document_produced: Option<String>,
    #[serde(default)]
    pub cron_expression: String,
    #[serde(default)]
    pub email_recipients: Vec<String>,
    #[serde(default)]
    pub executer_username: String,
}

/// One report definition.
///
/// `permission_label` is a list here, unlike `Election::permission_label`, which
/// is a single string. Both are matched against the administrator's
/// `permission_labels` attribute, and an entity carrying a label nobody holds is
/// invisible in the Admin Portal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub election_event_id: String,
    pub tenant_id: String,
    pub election_id: Option<String>,
    pub report_type: String,
    pub template_alias: Option<String>,
    pub encryption_policy: EReportEncryption,
    pub cron_config: Option<ReportCronConfig>,
    pub created_at: DateTime<Utc>,
    pub permission_label: Option<Vec<String>>,
    /// How many copies each generation prints. Absent means one.
    #[serde(default)]
    pub copies: Option<u32>,
    /// The formats each generation writes. Absent means the platform's
    /// default for the type.
    #[serde(default)]
    pub output_formats: Option<Vec<ReportFormat>>,
}

/// A format a report can be generated in.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ReportFormat {
    Pdf,
    Csv,
    Xml,
    Sql,
}

/// The `copies` cell of a reports CSV: empty or absent is `None`.
pub fn parse_copies(cell: Option<&str>) -> Result<Option<u32>, String> {
    match cell.map(str::trim).filter(|cell| !cell.is_empty()) {
        None => Ok(None),
        Some(text) => match text.parse::<u32>() {
            Ok(count) if count > 0 => Ok(Some(count)),
            _ => Err(format!(
                "copies must be a whole number of at least 1, not '{text}'"
            )),
        },
    }
}

/// The `output_formats` cell of a reports CSV, `|`-separated: empty or
/// absent is `None`.
pub fn parse_output_formats(
    cell: Option<&str>,
) -> Result<Option<Vec<ReportFormat>>, String> {
    match cell.map(str::trim).filter(|cell| !cell.is_empty()) {
        None => Ok(None),
        Some(text) => text
            .split(super::emit::MULTI_VALUE_SEPARATOR)
            .map(|name| {
                ReportFormat::from_str(name.trim())
                    .map_err(|_| format!("'{name}' is not a report format"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
    }
}

impl ReportType {
    /// The formats a report of this type can be generated in, the first
    /// being its default: PDF and XML for the election returns, PDF, CSV
    /// and SQL for the activity logs, and PDF for the rest.
    pub fn formats(&self) -> &'static [ReportFormat] {
        match self {
            ReportType::ELECTORAL_RESULTS => {
                &[ReportFormat::Pdf, ReportFormat::Xml]
            }
            ReportType::ACTIVITY_LOGS => {
                &[ReportFormat::Pdf, ReportFormat::Csv, ReportFormat::Sql]
            }
            ReportType::INITIALIZATION_REPORT
            | ReportType::BALLOT_IMAGES
            | ReportType::BALLOT_RECEIPT
            | ReportType::MANUAL_VERIFICATION
            | ReportType::PARTICIPATION_REPORT
            | ReportType::CREDENTIALS => &[ReportFormat::Pdf],
        }
    }
}

/// The kinds of report the platform can generate.
///
/// `Report::report_type` is a `String` rather than this enum because the column
/// is free text in the database; this is the set a writer should choose from.
#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum ReportType {
    INITIALIZATION_REPORT,
    ELECTORAL_RESULTS,
    BALLOT_IMAGES,
    BALLOT_RECEIPT,
    ACTIVITY_LOGS,
    MANUAL_VERIFICATION,
    PARTICIPATION_REPORT,
    CREDENTIALS,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn each_report_type_offers_its_formats_default_first() {
        assert_eq!(
            ReportType::ELECTORAL_RESULTS.formats(),
            &[ReportFormat::Pdf, ReportFormat::Xml]
        );
        assert_eq!(
            ReportType::ACTIVITY_LOGS.formats(),
            &[ReportFormat::Pdf, ReportFormat::Csv, ReportFormat::Sql]
        );
        assert_eq!(ReportType::BALLOT_RECEIPT.formats(), &[ReportFormat::Pdf]);
    }

    #[test]
    fn a_format_reads_and_writes_lowercase() {
        assert_eq!(ReportFormat::from_str("xml").unwrap(), ReportFormat::Xml);
        assert!(ReportFormat::from_str("docx").is_err());
        assert_eq!(
            serde_json::to_string(&ReportFormat::Pdf).unwrap(),
            "\"pdf\""
        );
    }

    #[test]
    fn the_csv_cells_read_back_what_the_builder_wrote() {
        assert_eq!(parse_copies(None), Ok(None));
        assert_eq!(parse_copies(Some("")), Ok(None));
        assert_eq!(parse_copies(Some("7")), Ok(Some(7)));
        assert!(parse_copies(Some("0")).is_err());
        assert!(parse_copies(Some("seven")).is_err());

        assert_eq!(parse_output_formats(Some("")), Ok(None));
        assert_eq!(
            parse_output_formats(Some("pdf|xml")),
            Ok(Some(vec![ReportFormat::Pdf, ReportFormat::Xml]))
        );
        assert!(parse_output_formats(Some("pdf|docx")).is_err());
    }

    #[test]
    fn a_report_without_copies_or_formats_still_reads() {
        let report: Report = serde_json::from_value(serde_json::json!({
            "id": "r",
            "election_event_id": "e",
            "tenant_id": "t",
            "election_id": null,
            "report_type": "ELECTORAL_RESULTS",
            "template_alias": null,
            "encryption_policy": "unencrypted",
            "cron_config": null,
            "created_at": "2026-01-01T00:00:00Z",
            "permission_label": null
        }))
        .unwrap();
        assert_eq!(report.copies, None);
        assert_eq!(report.output_formats, None);
    }

    #[test]
    fn encryption_policy_serializes_snake_case() {
        // The importer reads this straight out of a CSV column, so the wire form
        // is part of the file format rather than an implementation detail.
        assert_eq!(
            serde_json::to_string(&EReportEncryption::ConfiguredPassword)
                .unwrap(),
            "\"configured_password\""
        );
        assert_eq!(
            serde_json::to_string(&EReportEncryption::Unencrypted).unwrap(),
            "\"unencrypted\""
        );
    }

    #[test]
    fn encryption_policy_parses_from_the_csv_spelling() {
        assert_eq!(
            EReportEncryption::from_str("configured_password").unwrap(),
            EReportEncryption::ConfiguredPassword
        );
        assert!(EReportEncryption::from_str("generated_password").is_err());
    }

    #[test]
    fn cron_config_tolerates_an_empty_object() {
        // An absent key must not fail deserialization: most reports have no cron.
        let parsed: ReportCronConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(parsed, ReportCronConfig::default());
        assert!(!parsed.is_active);
    }

    #[test]
    fn cron_config_round_trips_the_shape_the_workbook_writes() {
        let source = r#"{"is_active":true,"last_document_produced":null,
            "cron_expression":"46 0 * * *","email_recipients":["ops@example.org"],
            "executer_username":"admin"}"#;
        let parsed: ReportCronConfig = serde_json::from_str(source).unwrap();
        assert!(parsed.is_active);
        assert_eq!(parsed.cron_expression, "46 0 * * *");
        assert_eq!(parsed.email_recipients, vec!["ops@example.org"]);
    }

    #[test]
    fn every_report_type_round_trips() {
        for name in [
            "INITIALIZATION_REPORT",
            "ELECTORAL_RESULTS",
            "BALLOT_IMAGES",
            "BALLOT_RECEIPT",
            "ACTIVITY_LOGS",
            "MANUAL_VERIFICATION",
            "PARTICIPATION_REPORT",
            "CREDENTIALS",
        ] {
            let parsed = ReportType::from_str(name)
                .unwrap_or_else(|_| panic!("{name} should be a ReportType"));
            assert_eq!(parsed.to_string(), name);
        }
    }

    #[test]
    fn permission_label_is_a_list() {
        // Election::permission_label is a single string; getting these the wrong
        // way round fails deserialization at import time.
        let source = r#"{"id":"a","election_event_id":"b","tenant_id":"c",
            "election_id":null,"report_type":"ACTIVITY_LOGS","template_alias":null,
            "encryption_policy":"unencrypted","cron_config":null,
            "created_at":"2026-01-01T00:00:00Z","permission_label":["x","y"]}"#;
        let parsed: Report = serde_json::from_str(source).unwrap();
        assert_eq!(parsed.permission_label, Some(vec!["x".into(), "y".into()]));
    }
}
