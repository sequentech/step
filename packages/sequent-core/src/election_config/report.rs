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
